//! Regression test for the `agents add-knowledge` P0 bug (v0.1.x).
//!
//! Before v0.2.0: the command POSTed to `/v1/convai/knowledge-base/{url|text|file}`
//! to create a KB document but NEVER called PATCH on the agent, so the
//! `agent_id` arg was effectively unused — agents never actually got the
//! knowledge attached. This test locks in the fix: POST the doc, GET the
//! agent, then PATCH the agent with the new doc appended to
//! `conversation_config.agent.prompt.knowledge_base`.

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

/// Responder that captures the request body so we can assert on it after
/// the CLI run completes.
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
async fn add_knowledge_creates_doc_and_patches_agent() {
    let mock = MockServer::start().await;
    let agent_id = "agent_abc123";
    let doc_id = "doc_xyz789";

    // 1. KB create returns the doc id + name + (implied) type=url.
    Mock::given(method("POST"))
        .and(path("/v1/convai/knowledge-base/url"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": doc_id,
            "name": "Docs Page",
            "prompt_injectable": true,
        })))
        .mount(&mock)
        .await;

    // 2. GET /v1/convai/agents/{id} — returns current config with an
    //    existing KB entry so we can assert we append (not overwrite).
    Mock::given(method("GET"))
        .and(path(format!("/v1/convai/agents/{agent_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "agent_id": agent_id,
            "name": "Sales Bot",
            "conversation_config": {
                "agent": {
                    "prompt": {
                        "prompt": "You sell widgets.",
                        "knowledge_base": [
                            { "id": "existing_doc", "type": "text", "name": "FAQ", "usage_mode": "auto" }
                        ]
                    }
                }
            }
        })))
        .mount(&mock)
        .await;

    // 3. PATCH the agent with the appended KB entry.
    let patch_body = Arc::new(Mutex::new(None));
    Mock::given(method("PATCH"))
        .and(path(format!("/v1/convai/agents/{agent_id}")))
        .respond_with(Recorder {
            body: patch_body.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "agent_id": agent_id,
                "updated": true,
            })),
        })
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "agents",
            "add-knowledge",
            agent_id,
            "Docs Page",
            "--url",
            "https://example.com/docs",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "add-knowledge should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The JSON envelope must surface the doc id and attached=true so
    // agents downstream can wire up the KB reference.
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["doc_id"], doc_id);
    assert_eq!(body["data"]["attached"], true);

    // The PATCH body must include the freshly-created doc entry inside
    // conversation_config.agent.prompt.knowledge_base, and preserve the
    // existing "FAQ" entry (i.e. append, don't overwrite).
    let patched = patch_body
        .lock()
        .unwrap()
        .clone()
        .expect("PATCH body captured");
    let kb = patched
        .pointer("/conversation_config/agent/prompt/knowledge_base")
        .and_then(|v| v.as_array())
        .expect("knowledge_base array must be in PATCH body");
    assert_eq!(kb.len(), 2, "existing FAQ + new Docs Page");
    // Preserved entry.
    assert_eq!(kb[0]["id"], "existing_doc");
    // Newly-attached entry.
    assert_eq!(kb[1]["id"], doc_id);
    assert_eq!(kb[1]["type"], "url");
    assert_eq!(kb[1]["name"], "Docs Page");
    assert_eq!(kb[1]["usage_mode"], "auto");
}

#[tokio::test(flavor = "multi_thread")]
async fn add_knowledge_surfaces_doc_id_when_patch_fails() {
    let mock = MockServer::start().await;
    let agent_id = "agent_abc123";
    let doc_id = "doc_created_but_orphan";

    // KB create succeeds.
    Mock::given(method("POST"))
        .and(path("/v1/convai/knowledge-base/text"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": doc_id,
            "name": "Orphan",
        })))
        .mount(&mock)
        .await;

    // Agent GET succeeds with an empty KB.
    Mock::given(method("GET"))
        .and(path(format!("/v1/convai/agents/{agent_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "agent_id": agent_id,
            "conversation_config": { "agent": { "prompt": { "knowledge_base": [] } } }
        })))
        .mount(&mock)
        .await;

    // PATCH fails with 500 — the CLI must include the doc id in the error
    // so the user can retry just the attach without recreating the doc.
    Mock::given(method("PATCH"))
        .and(path(format!("/v1/convai/agents/{agent_id}")))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "detail": { "message": "internal error" }
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "agents",
            "add-knowledge",
            agent_id,
            "Orphan",
            "--text",
            "some content",
        ])
        .output()
        .unwrap();

    assert!(!out.status.success(), "should fail");
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    let msg = err["error"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains(doc_id),
        "error message must surface the created doc id so the user can retry attach; got: {msg}"
    );
}
