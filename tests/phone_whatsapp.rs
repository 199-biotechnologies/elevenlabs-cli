//! HTTP routing + body shape contract for `elevenlabs phone whatsapp …`
//! (call, message, and the accounts CRUD surface).
//!
//! Body shapes are grounded against
//! https://github.com/elevenlabs/elevenlabs-python/blob/main/src/elevenlabs/conversational_ai/whatsapp/raw_client.py
//! — if these tests fail, diff against that file to see what the SDK
//! actually sends.

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
async fn whatsapp_call_posts_outbound_call_with_permission_template() {
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
            "agent_1",
            "--whatsapp-phone-number",
            "wa_phone_1",
            "--whatsapp-user",
            "wa_user_1",
            "--permission-template",
            "call_consent_v1",
            "--permission-template-language",
            "en_US",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "whatsapp call failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = captured.lock().unwrap().clone().expect("no body captured");
    assert_eq!(body["agent_id"], "agent_1");
    assert_eq!(body["whatsapp_phone_number_id"], "wa_phone_1");
    assert_eq!(body["whatsapp_user_id"], "wa_user_1");
    assert_eq!(
        body["whatsapp_call_permission_request_template_name"],
        "call_consent_v1"
    );
    assert_eq!(
        body["whatsapp_call_permission_request_template_language_code"],
        "en_US"
    );
    assert!(body.get("whatsapp_account_id").is_none());
    assert!(body.get("recipient_phone_number").is_none());
}

// ── whatsapp message (template only — no free-form text) ───────────────────

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_message_template_body_shape() {
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
            "agent_1",
            "--whatsapp-phone-number",
            "wa_phone_1",
            "--whatsapp-user",
            "wa_user_1",
            "--template",
            "welcome_en",
            "--template-language",
            "en_US",
            "--template-param",
            "name=Alice",
            "--template-param",
            "code=1234",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "whatsapp message (template) failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = captured.lock().unwrap().clone().expect("no body captured");
    assert_eq!(body["agent_id"], "agent_1");
    assert_eq!(body["whatsapp_phone_number_id"], "wa_phone_1");
    assert_eq!(body["whatsapp_user_id"], "wa_user_1");
    assert_eq!(body["template_name"], "welcome_en");
    assert_eq!(body["template_language_code"], "en_US");

    let params = body["template_params"]
        .as_array()
        .expect("template_params is an array");
    assert_eq!(params.len(), 1);
    assert_eq!(params[0]["type"], "body");
    let inner = params[0]["parameters"]
        .as_array()
        .expect("parameters is an array");
    assert_eq!(inner.len(), 2);
    assert_eq!(inner[0]["parameter_name"], "name");
    assert_eq!(inner[0]["type"], "text");
    assert_eq!(inner[0]["text"], "Alice");
    assert_eq!(inner[1]["parameter_name"], "code");
    assert_eq!(inner[1]["text"], "1234");

    assert!(body.get("text").is_none());
    assert!(body.get("whatsapp_account_id").is_none());
    assert!(body.get("recipient_phone_number").is_none());
    assert!(body.get("conversation_initiation_client_data").is_none());
}

// ── whatsapp message with --client-data pass-through ───────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_message_with_client_data_file_passes_through() {
    let mock = MockServer::start().await;
    let captured: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/convai/whatsapp/outbound-message"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message_id": "m_cd",
                "status": "queued",
            })),
        })
        .mount(&mock)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let cd = tmp.path().join("client_data.json");
    std::fs::write(
        &cd,
        r#"{"dynamic_variables":{"first_name":"Alice"},"source":"cli"}"#,
    )
    .unwrap();

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
            "agent_1",
            "--whatsapp-phone-number",
            "wa_phone_1",
            "--whatsapp-user",
            "wa_user_1",
            "--template",
            "welcome_en",
            "--template-language",
            "en_US",
            "--client-data",
            cd.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "whatsapp message with client-data failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = captured.lock().unwrap().clone().expect("no body captured");
    assert_eq!(
        body["conversation_initiation_client_data"]["dynamic_variables"]["first_name"],
        "Alice"
    );
    assert_eq!(body["conversation_initiation_client_data"]["source"], "cli");
}

// ── whatsapp message rejects bad template-param shape ──────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn whatsapp_message_rejects_bad_template_param() {
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
            "--whatsapp-phone-number",
            "p",
            "--whatsapp-user",
            "u",
            "--template",
            "t",
            "--template-language",
            "en_US",
            "--template-param",
            "no_equals_sign",
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "expected exit 3 (invalid_input)"
    );
}

// ── accounts CRUD (unchanged from pre-fix; kept to pin regressions) ────────

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
