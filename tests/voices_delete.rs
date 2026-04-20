//! `voices delete` — destructive-op guard tests.
//!
//! The `--yes` guard existed in code before v0.2 but had no coverage.
//! These tests pin the refuse-without-yes exit contract (3) and the
//! happy-path DELETE against the server.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn bin() -> AssertCmd {
    AssertCmd::cargo_bin("elevenlabs").unwrap()
}

fn temp_config_with_key(api_key: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, path)
}

/// Without `--yes`, refuse. Voice deletion is irreversible.
#[tokio::test(flavor = "multi_thread")]
async fn voices_delete_without_yes_is_invalid_input() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["voices", "delete", "v_del"])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(3),
        "no --yes → invalid_input (exit 3); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "invalid_input");
    let message = err["error"]["message"].as_str().unwrap_or("");
    assert!(
        message.contains("--yes"),
        "error must mention --yes; got: {message}"
    );
}

/// With `--yes`, issue `DELETE /v1/voices/{id}` and return a success
/// envelope carrying `deleted: true`.
#[tokio::test(flavor = "multi_thread")]
async fn voices_delete_with_yes_hits_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/voices/v_del"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["voices", "delete", "v_del", "--yes"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "voices delete --yes should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["deleted"], true);
    assert_eq!(body["data"]["voice_id"], "v_del");
}
