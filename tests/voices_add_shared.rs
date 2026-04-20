//! Verifies `voices add-shared` hits
//! `POST /v1/voices/add/{public_user_id}/{voice_id}` with the correct JSON
//! body and parses the response into the success envelope.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;
use wiremock::matchers::{body_json, header, method, path};
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

#[tokio::test(flavor = "multi_thread")]
async fn add_shared_interpolates_path_params_and_sends_body() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/voices/add/pub_USER_ABC/voice_XYZ"))
        .and(header("xi-api-key", "sk_test_keyyyyyyyyy"))
        .and(body_json(serde_json::json!({
            "new_name": "Morgan (copy)",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voice_id": "lib_copy_XYZ",
            "name": "Morgan (copy)"
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "voices",
            "add-shared",
            "pub_USER_ABC",
            "voice_XYZ",
            "--name",
            "Morgan (copy)",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["voice_id"], "lib_copy_XYZ");
    assert_eq!(body["data"]["name"], "Morgan (copy)");
}

#[tokio::test(flavor = "multi_thread")]
async fn add_shared_sends_bookmarked_when_set() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/voices/add/pub_u/v_1"))
        .and(body_json(serde_json::json!({
            "new_name": "Test",
            "bookmarked": true,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voice_id": "vcopy",
            "name": "Test"
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "voices",
            "add-shared",
            "pub_u",
            "v_1",
            "--name",
            "Test",
            "--bookmarked",
            "true",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn add_shared_requires_name() {
    // Missing --name should fail at clap parse — exit 3.
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["voices", "add-shared", "pub_u", "v_1"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "missing --name must exit 3");
}
