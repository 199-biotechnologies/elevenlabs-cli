//! Integration tests for `agents update` and `agents duplicate`.

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
async fn agents_update_passes_json_through_verbatim() {
    let mock = MockServer::start().await;
    let agent_id = "agent_upd1";

    let captured = Arc::new(Mutex::new(None));
    Mock::given(method("PATCH"))
        .and(path(format!("/v1/convai/agents/{agent_id}")))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "agent_id": agent_id,
                "updated": true,
            })),
        })
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");

    // Write a patch file: nested update to the system prompt.
    let tmp = tempfile::tempdir().unwrap();
    let patch_path = tmp.path().join("patch.json");
    std::fs::write(
        &patch_path,
        r#"{"conversation_config":{"agent":{"prompt":{"prompt":"You are helpful."}}},"name":"Renamed"}"#,
    )
    .unwrap();

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "agents",
            "update",
            agent_id,
            "--patch",
            patch_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "update should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");

    let sent = captured
        .lock()
        .unwrap()
        .clone()
        .expect("captured PATCH body");
    assert_eq!(sent["name"], "Renamed");
    assert_eq!(
        sent["conversation_config"]["agent"]["prompt"]["prompt"],
        "You are helpful."
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_update_rejects_missing_patch_file() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "agents",
            "update",
            "agent_nope",
            "--patch",
            "/tmp/definitely-does-not-exist-xyz.json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "missing file → exit 3");
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "invalid_input");
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_duplicate_hits_duplicate_endpoint_with_name_override() {
    let mock = MockServer::start().await;
    let agent_id = "agent_src";
    let new_id = "agent_copy";

    let captured = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path(format!("/v1/convai/agents/{agent_id}/duplicate")))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "agent_id": new_id,
            })),
        })
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["agents", "duplicate", agent_id, "--name", "Clone A"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "duplicate should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["agent_id"], new_id);

    let sent = captured.lock().unwrap().clone().expect("captured body");
    assert_eq!(sent["name"], "Clone A");
}
