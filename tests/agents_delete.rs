//! `agents delete` — destructive-op guard tests.
//!
//! Mirrors `tests/agents_tools.rs` for the tool-delete guard: without
//! `--yes` we must exit 3 (invalid_input) before touching the network;
//! with `--yes` we issue `DELETE /v1/convai/agents/{id}` and return a
//! success envelope carrying `deleted: true`.

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

/// Without `--yes`, refuse. The agent-delete op is irreversible and
/// cascades (conversations + KB entries + tool-dep edges disappear with
/// the agent), so we must exit 3 before hitting the server.
#[tokio::test(flavor = "multi_thread")]
async fn delete_without_yes_is_invalid_input() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["agents", "delete", "agent_xyz"])
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
    // The suggestion should be the exact command to re-run.
    let suggestion = err["error"]["suggestion"].as_str().unwrap_or("");
    assert!(
        suggestion.contains("--yes") && suggestion.contains("agent_xyz"),
        "suggestion must include the --yes retry command; got: {suggestion}"
    );
}

/// With `--yes`, hit `DELETE /v1/convai/agents/{id}` and return a
/// success envelope carrying `deleted: true`.
#[tokio::test(flavor = "multi_thread")]
async fn delete_with_yes_hits_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/convai/agents/agent_xyz"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["agents", "delete", "agent_xyz", "--yes"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "agents delete --yes should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["deleted"], true);
    assert_eq!(body["data"]["agent_id"], "agent_xyz");
}
