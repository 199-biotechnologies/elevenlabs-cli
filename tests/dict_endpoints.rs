//! HTTP routing contract for `elevenlabs dict …`.
//!
//! For every subcommand we wire a wiremock that ONLY matches the expected
//! method + path. If the CLI regresses to a wrong path or verb, the mock
//! won't match → wiremock returns 404 → the test fails fast.
//!
//! We deliberately do NOT assert on response shape here — that belongs in
//! the richer per-feature tests. This file is the "does it reach the right
//! endpoint?" smoke test.

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

// ── list ───────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dict_list_hits_v1_pronunciation_dictionaries() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/pronunciation-dictionaries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "pronunciation_dictionaries": [
                {"id": "pd_1", "name": "Demo", "latest_version_id": "v1", "latest_version_rules_num": 2},
            ],
            "has_more": false,
            "next_cursor": null,
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dict", "list", "--search", "demo", "--page-size", "10"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "dict list failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["pronunciation_dictionaries"][0]["id"], "pd_1");
}

// ── add-file (multipart) ───────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dict_add_file_posts_multipart_to_add_from_file() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/pronunciation-dictionaries/add-from-file"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pd_new",
            "name": "MyDict",
            "version_id": "v1",
        })))
        .mount(&mock)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let pls = tmp.path().join("dict.pls");
    std::fs::write(
        &pls,
        br#"<?xml version="1.0"?><lexicon version="1.0" alphabet="ipa" xml:lang="en-US"></lexicon>"#,
    )
    .unwrap();

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dict",
            "add-file",
            "MyDict",
            pls.to_str().unwrap(),
            "--description",
            "hello",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "add-file failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["id"], "pd_new");
}

// ── add-rules (in-line JSON) ───────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dict_add_rules_posts_to_add_from_rules() {
    let mock = MockServer::start().await;
    let captured = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/pronunciation-dictionaries/add-from-rules"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "pd_r1",
                "name": "Rules",
                "version_id": "v1",
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
            "dict",
            "add-rules",
            "Rules",
            "--rule",
            "tomato:təˈmɑːtoʊ",
            "--alias-rule",
            "NASA:nah-suh",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "add-rules failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["id"], "pd_r1");

    let sent = captured
        .lock()
        .unwrap()
        .clone()
        .expect("captured add-from-rules body");
    assert_eq!(sent["name"], "Rules");
    let rules = sent["rules"].as_array().expect("rules array");
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0]["type"], "phoneme");
    assert_eq!(rules[0]["string_to_replace"], "tomato");
    assert_eq!(rules[0]["phoneme"], "təˈmɑːtoʊ");
    assert_eq!(rules[0]["alphabet"], "ipa");
    assert_eq!(rules[1]["type"], "alias");
    assert_eq!(rules[1]["string_to_replace"], "NASA");
    assert_eq!(rules[1]["alias"], "nah-suh");
}

// ── show ───────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dict_show_hits_by_id() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/pronunciation-dictionaries/pd_abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pd_abc",
            "name": "Demo",
            "latest_version_id": "v9",
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dict", "show", "pd_abc"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "show failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["id"], "pd_abc");
}

// ── update (PATCH) ─────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dict_update_patches_with_archive_flag() {
    let mock = MockServer::start().await;
    let captured = Arc::new(Mutex::new(None));
    Mock::given(method("PATCH"))
        .and(path("/v1/pronunciation-dictionaries/pd_up"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "pd_up",
                "name": "Renamed",
                "archived_time_unix": 1700000000u64,
            })),
        })
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dict", "update", "pd_up", "--name", "Renamed", "--archive"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "update failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sent = captured
        .lock()
        .unwrap()
        .clone()
        .expect("captured PATCH body");
    assert_eq!(sent["name"], "Renamed");
    assert_eq!(sent["archived"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn dict_update_rejects_empty_patch() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dict", "update", "pd_up"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "empty update must be exit 3; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "invalid_input");
}

// ── set-rules ──────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dict_set_rules_posts_to_set_rules_path() {
    let mock = MockServer::start().await;
    let captured = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/pronunciation-dictionaries/pd_sr/set-rules"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "pd_sr",
                "version_id": "v2",
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
            "dict",
            "set-rules",
            "pd_sr",
            "--rule",
            "ok:oʊˈkeɪ",
            "--case-sensitive",
            "true",
            "--word-boundaries",
            "false",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "set-rules failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sent = captured.lock().unwrap().clone().expect("captured body");
    let rules = sent["rules"].as_array().expect("rules array");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["string_to_replace"], "ok");
    assert_eq!(sent["case_sensitive"], true);
    assert_eq!(sent["word_boundaries"], false);
}

// ── add-rules-to ───────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dict_add_rules_to_posts_to_add_rules_path() {
    let mock = MockServer::start().await;
    let captured = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/pronunciation-dictionaries/pd_art/add-rules"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "pd_art",
                "version_id": "v3",
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
            "dict",
            "add-rules-to",
            "pd_art",
            "--alias-rule",
            "SCUBA:scoo-buh",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "add-rules-to failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sent = captured.lock().unwrap().clone().expect("captured body");
    let rules = sent["rules"].as_array().expect("rules array");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["type"], "alias");
    assert_eq!(rules[0]["alias"], "scoo-buh");
}

// ── remove-rules ───────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dict_remove_rules_posts_rule_strings() {
    let mock = MockServer::start().await;
    let captured = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/pronunciation-dictionaries/pd_rm/remove-rules"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "pd_rm",
                "version_id": "v4",
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
            "dict",
            "remove-rules",
            "pd_rm",
            "--word",
            "tomato",
            "--word",
            "potato",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "remove-rules failed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sent = captured.lock().unwrap().clone().expect("captured body");
    let words = sent["rule_strings"].as_array().expect("rule_strings");
    assert_eq!(words.len(), 2);
    assert_eq!(words[0], "tomato");
    assert_eq!(words[1], "potato");
}

#[tokio::test(flavor = "multi_thread")]
async fn dict_remove_rules_requires_word() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dict", "remove-rules", "pd_rm"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "missing --word must exit 3; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
