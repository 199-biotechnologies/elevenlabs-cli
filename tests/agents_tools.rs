//! Integration tests for `agents tools` subcommand family.

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
async fn agents_tools_list_hits_correct_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/convai/tools"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tools": [
                {"id": "tool_1", "tool_config": {"name": "calendar", "type": "webhook"}}
            ]
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["agents", "tools", "list"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "tools list should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["tools"][0]["id"], "tool_1");
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_tools_create_posts_json_body_verbatim() {
    let mock = MockServer::start().await;
    let captured = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/convai/tools"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "tool_new",
                "tool_config": { "name": "my_tool", "type": "client" },
            })),
        })
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp = tempfile::tempdir().unwrap();
    let cfg_path = tmp.path().join("tool.json");
    std::fs::write(
        &cfg_path,
        r#"{"tool_config":{"name":"my_tool","type":"client","description":"d"}}"#,
    )
    .unwrap();

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "agents",
            "tools",
            "create",
            "--config",
            cfg_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "tools create should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sent = captured.lock().unwrap().clone().expect("captured body");
    assert_eq!(sent["tool_config"]["name"], "my_tool");
    assert_eq!(sent["tool_config"]["type"], "client");
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_tools_delete_requires_yes_flag() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["agents", "tools", "delete", "tool_1"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "no --yes → invalid_input");
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "invalid_input");
    assert!(
        err["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("--yes"),
        "error must mention --yes"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_tools_delete_with_yes_calls_delete_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/convai/tools/tool_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&mock)
        .await;
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["agents", "tools", "delete", "tool_1", "--yes"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "tools delete --yes should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["deleted"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_tools_deps_hits_dependent_agents_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/convai/tools/tool_1/dependent-agents"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "agents": [
                {"agent_id": "a1", "name": "Bot A"},
                {"agent_id": "a2", "name": "Bot B"},
            ]
        })))
        .mount(&mock)
        .await;
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["agents", "tools", "deps", "tool_1"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "deps should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["agents"][0]["agent_id"], "a1");
    assert_eq!(body["data"]["agents"][1]["agent_id"], "a2");
}
