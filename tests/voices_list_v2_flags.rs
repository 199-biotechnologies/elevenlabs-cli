//! Verifies `voices list` forwards the full v2 query-param set (Apr 13, 2026
//! additions): voice_type, category, fine_tuning_state, collection_id,
//! include_total_count, next_page_token, voice_ids (repeatable).
//!
//! Each assertion uses wiremock's query_param matcher so we lock in the
//! exact wire contract.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;
use wiremock::matchers::{method, path, query_param};
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
async fn list_forwards_voice_type_non_community() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .and(query_param("voice_type", "non-community"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": [
                {"voice_id": "v1", "name": "Personal", "category": "cloned"}
            ]
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["voices", "list", "--voice-type", "non-community"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
}

#[tokio::test(flavor = "multi_thread")]
async fn list_forwards_full_v2_filter_set() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .and(query_param("category", "professional"))
        .and(query_param("fine_tuning_state", "fine_tuned"))
        .and(query_param("collection_id", "col_42"))
        .and(query_param("include_total_count", "true"))
        .and(query_param("next_page_token", "tok_ABC"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": [],
            "has_more": false,
            "total_count": 0
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
            "list",
            "--category",
            "professional",
            "--fine-tuning-state",
            "fine_tuned",
            "--collection-id",
            "col_42",
            "--include-total-count",
            "--next-page-token",
            "tok_ABC",
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
async fn list_repeats_voice_ids_query_param() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .and(query_param("voice_ids", "vid_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": []
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
            "list",
            "--voice-id",
            "vid_1",
            "--voice-id",
            "vid_2",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Verify both values went out on the wire (we can't use query_param
    // twice on the matcher because wiremock treats it as AND over the same
    // key; instead, inspect the received request).
    let reqs = mock.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    let url = reqs[0].url.to_string();
    assert!(
        url.contains("voice_ids=vid_1") && url.contains("voice_ids=vid_2"),
        "expected both voice_ids in url; got {url}"
    );
}
