//! `phone batch submit` CSV + JSON recipient parsing + request body shape.
//!
//! These tests exercise the CSV/JSON recipient loader and verify that the
//! POST body sent to `/v1/convai/batch-calling/submit` matches the SDK:
//!
//!   {
//!     "agent_id": "...",
//!     "agent_phone_number_id": "...",
//!     "recipients": [{"phone_number": "+1...", "conversation_initiation_client_data": {...}?}, ...],
//!     "call_name"?: "...",                  // SDK field is `call_name`, NOT `name`
//!     "scheduled_time_unix"?: 1234567890
//!   }
//!
//! Grounded against:
//! https://github.com/elevenlabs/elevenlabs-python/blob/main/src/elevenlabs/conversational_ai/batch_calls/raw_client.py
//! (the `create` method's JSON body).

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

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

struct Recorder {
    body: Arc<Mutex<Option<serde_json::Value>>>,
    response: ResponseTemplate,
}

impl Respond for Recorder {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&req.body) {
            *self.body.lock().unwrap() = Some(v);
        }
        self.response.clone()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn submit_csv_basic_body_shape() {
    let mock = MockServer::start().await;
    let captured: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/convai/batch-calling/submit"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "batch_abc",
                "status": "queued",
            })),
        })
        .mount(&mock)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let csv = tmp.path().join("r.csv");
    std::fs::write(
        &csv,
        "phone_number,conversation_initiation_client_data\n\
         +14155550001,\"{\"\"dynamic_variables\"\":{\"\"name\"\":\"\"Alice\"\"}}\"\n\
         +14155550002,\n",
    )
    .unwrap();

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "phone",
            "batch",
            "submit",
            "--agent",
            "agent_1",
            "--phone-number",
            "phone_1",
            "--recipients",
            csv.to_str().unwrap(),
            "--name",
            "My Batch",
            "--scheduled-time-unix",
            "1700000000",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "batch submit failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = captured.lock().unwrap().clone().expect("no body captured");
    assert_eq!(body["agent_id"], "agent_1");
    assert_eq!(body["agent_phone_number_id"], "phone_1");
    // SDK field name is `call_name` (not `name`).
    assert_eq!(body["call_name"], "My Batch");
    assert!(
        body.get("name").is_none(),
        "stale `name` field must be gone"
    );
    assert_eq!(body["scheduled_time_unix"], 1700000000);
    let recipients = body["recipients"].as_array().expect("recipients array");
    assert_eq!(recipients.len(), 2);
    assert_eq!(recipients[0]["phone_number"], "+14155550001");
    assert_eq!(
        recipients[0]["conversation_initiation_client_data"]["dynamic_variables"]["name"],
        "Alice"
    );
    assert_eq!(recipients[1]["phone_number"], "+14155550002");
    assert!(
        recipients[1]
            .get("conversation_initiation_client_data")
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn submit_json_array_body_shape() {
    let mock = MockServer::start().await;
    let captured: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/convai/batch-calling/submit"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "batch_json",
                "status": "queued",
            })),
        })
        .mount(&mock)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let json = tmp.path().join("r.json");
    std::fs::write(
        &json,
        r#"[
            {"phone_number":"+14155550003","conversation_initiation_client_data":{"k":"v"}},
            {"phone_number":"+14155550004"}
        ]"#,
    )
    .unwrap();

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "phone",
            "batch",
            "submit",
            "--agent",
            "agent_2",
            "--phone-number",
            "phone_2",
            "--recipients",
            json.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "batch submit (json) failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = captured.lock().unwrap().clone().expect("no body captured");
    let recipients = body["recipients"].as_array().expect("recipients array");
    assert_eq!(recipients.len(), 2);
    assert_eq!(
        recipients[0]["conversation_initiation_client_data"]["k"],
        "v"
    );
    // Neither `call_name` nor the stale `name` should be present — the
    // test did not pass `--name`.
    assert!(body.get("call_name").is_none());
    assert!(body.get("name").is_none());
    assert!(body.get("scheduled_time_unix").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn submit_missing_file_is_invalid_input() {
    // No mock — we should never reach the network.
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        // Use an unreachable URL to guarantee the test fails fast if it
        // *does* hit the network.
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "phone",
            "batch",
            "submit",
            "--agent",
            "a",
            "--phone-number",
            "p",
            "--recipients",
            "/does/not/exist.csv",
        ])
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
async fn submit_bad_json_column_is_invalid_input() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp = tempfile::tempdir().unwrap();
    let csv = tmp.path().join("bad.csv");
    std::fs::write(&csv, "+14155550009,{not valid json}\n").unwrap();

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "phone",
            "batch",
            "submit",
            "--agent",
            "a",
            "--phone-number",
            "p",
            "--recipients",
            csv.to_str().unwrap(),
        ])
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
