//! HTTP routing + body shape contract for `elevenlabs phone whatsapp …`
//! (call, message, and the accounts CRUD surface).

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

// ── whatsapp call ──────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_call_posts_outbound_call() {
    let mock = MockServer::start().await;
    let captured: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/convai/whatsapp/outbound-call"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "call_id": "wa_call_1",
                "status": "queued",
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
            "phone",
            "whatsapp",
            "call",
            "--agent",
            "a1",
            "--whatsapp-account",
            "wa1",
            "--recipient",
            "+14155550001",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "whatsapp call failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = captured.lock().unwrap().clone().expect("no body captured");
    assert_eq!(body["agent_id"], "a1");
    assert_eq!(body["whatsapp_account_id"], "wa1");
    assert_eq!(body["recipient_phone_number"], "+14155550001");
}

// ── whatsapp message (text) ────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_message_text_body_shape() {
    let mock = MockServer::start().await;
    let captured: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/convai/whatsapp/outbound-message"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message_id": "m_1",
                "status": "queued",
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
            "phone",
            "whatsapp",
            "message",
            "--agent",
            "a1",
            "--whatsapp-account",
            "wa1",
            "--recipient",
            "+14155550002",
            "--text",
            "Hello there",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "whatsapp message failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = captured.lock().unwrap().clone().expect("no body captured");
    assert_eq!(body["text"], "Hello there");
    assert!(body.get("template_name").is_none());
}

// ── whatsapp message (template) ────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_message_template_body_shape() {
    let mock = MockServer::start().await;
    let captured: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/convai/whatsapp/outbound-message"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message_id": "m_2",
                "status": "queued",
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
            "phone",
            "whatsapp",
            "message",
            "--agent",
            "a1",
            "--whatsapp-account",
            "wa1",
            "--recipient",
            "+14155550003",
            "--template",
            "welcome_en",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "whatsapp message (template) failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = captured.lock().unwrap().clone().expect("no body captured");
    assert_eq!(body["template_name"], "welcome_en");
    assert!(body.get("text").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_message_without_text_or_template_is_invalid_input() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "phone",
            "whatsapp",
            "message",
            "--agent",
            "a",
            "--whatsapp-account",
            "wa",
            "--recipient",
            "+14155550000",
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "expected exit 3 (invalid_input)"
    );
}

// ── accounts CRUD ──────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_accounts_list_hits_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/convai/whatsapp-accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "whatsapp_accounts": [
                {"id": "wa1", "display_name": "Sales", "phone_number": "+15555550001", "status": "active"},
            ],
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["phone", "whatsapp", "accounts", "list"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "accounts list failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["whatsapp_accounts"][0]["id"], "wa1");
}

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_accounts_show_hits_id_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/convai/whatsapp-accounts/wa1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "wa1",
            "display_name": "Sales",
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["phone", "whatsapp", "accounts", "show", "wa1"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "accounts show failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["id"], "wa1");
}

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_accounts_update_patches_with_body() {
    let mock = MockServer::start().await;
    let captured: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    Mock::given(method("PATCH"))
        .and(path("/v1/convai/whatsapp-accounts/wa1"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "wa1",
                "display_name": "Support",
            })),
        })
        .mount(&mock)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let patch = tmp.path().join("p.json");
    std::fs::write(&patch, r#"{"display_name":"Support"}"#).unwrap();

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "phone",
            "whatsapp",
            "accounts",
            "update",
            "wa1",
            "--patch",
            patch.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "accounts update failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = captured.lock().unwrap().clone().expect("no body captured");
    assert_eq!(body["display_name"], "Support");
}

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_accounts_delete_without_yes_is_invalid_input() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args(["phone", "whatsapp", "accounts", "delete", "wa1"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
}

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_accounts_delete_with_yes_hits_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/convai/whatsapp-accounts/wa1"))
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
        .args(["phone", "whatsapp", "accounts", "delete", "wa1", "--yes"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "accounts delete failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
