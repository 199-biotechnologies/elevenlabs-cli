//! Verifies `voices edit` handles name/description/labels via multipart
//! (`POST /v1/voices/{id}/edit`) and dispatches `--remove-sample` to
//! the dedicated `DELETE /v1/voices/{id}/samples/{sample_id}` endpoint.

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

#[tokio::test(flavor = "multi_thread")]
async fn edit_renames_voice_via_multipart() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/voices/v_123/edit"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "voice_id": "v_123",
            "name": "Renamed"
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["voices", "edit", "v_123", "--name", "Renamed"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["voice_id"], "v_123");

    let reqs = mock.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    let multipart = String::from_utf8_lossy(&reqs[0].body);
    assert!(multipart.contains("name=\"name\""));
    assert!(multipart.contains("Renamed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_serialises_labels_as_json_object_form_field() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/voices/v_abc/edit"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok"
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
            "edit",
            "v_abc",
            "--labels",
            "accent=british",
            "--labels",
            "gender=female",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let reqs = mock.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    let multipart = String::from_utf8_lossy(&reqs[0].body);
    assert!(multipart.contains("name=\"labels\""));
    assert!(multipart.contains("\"accent\":\"british\""));
    assert!(multipart.contains("\"gender\":\"female\""));
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_remove_sample_calls_delete_samples_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/voices/v_x/samples/sample_a"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok"
        })))
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v1/voices/v_x/samples/sample_b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok"
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
            "edit",
            "v_x",
            "--remove-sample",
            "sample_a",
            "--remove-sample",
            "sample_b",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let removed = body["data"]["removed_samples"].as_array().unwrap();
    assert_eq!(removed.len(), 2);
    assert_eq!(removed[0], "sample_a");
    assert_eq!(removed[1], "sample_b");
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_errors_without_any_changes() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["voices", "edit", "v_nothing"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "edit with no changes must exit 3 (invalid_input); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_labels_bad_format_rejected() {
    let mock = MockServer::start().await;
    // This mock should never be reached.
    Mock::given(method("POST"))
        .and(path("/v1/voices/v1/edit"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&mock)
        .await;
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["voices", "edit", "v1", "--labels", "no_equals_here"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "malformed --labels must exit 3 (invalid_input); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
