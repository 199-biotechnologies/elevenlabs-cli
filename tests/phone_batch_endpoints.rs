//! HTTP routing contract for `elevenlabs phone batch …`.
//!
//! For every batch sub-subcommand we wire a wiremock that ONLY matches the
//! expected method + path. If the CLI regresses to a wrong path or verb, the
//! mock won't match → wiremock returns 404 → the test fails fast.

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

// ── list ───────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn batch_list_hits_workspace_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/convai/batch-calling/workspace"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "batch_calls": [
                {"id": "batch_1", "name": "Demo", "status": "queued", "agent_id": "a1"},
            ],
            "next_cursor": "c_next"
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "phone",
            "batch",
            "list",
            "--page-size",
            "10",
            "--status",
            "queued",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "batch list failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["batch_calls"][0]["id"], "batch_1");
}

// ── show ───────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn batch_show_hits_id_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/convai/batch-calling/batch_abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "batch_abc",
            "status": "in_progress",
            "calls": [
                {"call_id": "c1", "status": "completed"},
                {"call_id": "c2", "status": "pending"},
            ],
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["phone", "batch", "show", "batch_abc"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "batch show failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["id"], "batch_abc");
    assert_eq!(body["data"]["calls"].as_array().unwrap().len(), 2);
}

// ── cancel ─────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn batch_cancel_posts_cancel_endpoint_no_yes_required() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/convai/batch-calling/batch_abc/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "batch_abc",
            "status": "cancelled",
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["phone", "batch", "cancel", "batch_abc"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "batch cancel failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["cancelled"], true);
}

// ── retry ──────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn batch_retry_posts_retry_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/convai/batch-calling/batch_abc/retry"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "batch_abc",
            "status": "queued",
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["phone", "batch", "retry", "batch_abc"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "batch retry failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["retried"], true);
}

// ── delete ─────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn batch_delete_without_yes_is_invalid_input() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        // Unreachable URL: confirm we never touch the network.
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args(["phone", "batch", "delete", "batch_abc"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "expected exit 3 (invalid_input)"
    );
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "invalid_input");
}

#[tokio::test(flavor = "multi_thread")]
async fn batch_delete_with_yes_hits_delete_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/convai/batch-calling/batch_abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "deleted": true
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["phone", "batch", "delete", "batch_abc", "--yes"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "batch delete failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["deleted"], true);
}
