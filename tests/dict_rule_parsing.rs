//! `parse_rule` edge cases, exercised end-to-end through `dict add-rules`.
//!
//! The parser is an internal helper in `src/commands/dict/mod.rs`. Since this
//! is a binary-only crate, we validate its contract through the CLI surface:
//! we pop `--rule` / `--alias-rule` values in, capture the HTTP body, and
//! assert the JSON shape.

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

async fn run_add_rules(
    flags: &[&str],
) -> (std::process::Output, Arc<Mutex<Option<serde_json::Value>>>) {
    let mock = MockServer::start().await;
    let captured = Arc::new(Mutex::new(None));
    Mock::given(method("POST"))
        .and(path("/v1/pronunciation-dictionaries/add-from-rules"))
        .respond_with(Recorder {
            body: captured.clone(),
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "pd_test",
                "name": "RuleParse",
                "version_id": "v1",
            })),
        })
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let mut args: Vec<&str> = vec!["dict", "add-rules", "RuleParse"];
    args.extend_from_slice(flags);

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(args)
        .output()
        .unwrap();
    (out, captured)
}

// ── phoneme-rule shape ────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn simple_phoneme_rule_rounds_trip() {
    let (out, captured) = run_add_rules(&["--rule", "cat:kæt"]).await;
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sent = captured.lock().unwrap().clone().expect("body");
    let rules = sent["rules"].as_array().unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["string_to_replace"], "cat");
    assert_eq!(rules[0]["phoneme"], "kæt");
    assert_eq!(rules[0]["type"], "phoneme");
    assert_eq!(rules[0]["alphabet"], "ipa");
}

#[tokio::test(flavor = "multi_thread")]
async fn phoneme_with_embedded_colons_survives_first_split() {
    // IPA length marks use `ː` (U+02D0) and some transcriptions use `:` as an
    // ASCII fallback for length. The first `:` must be the only split point,
    // so the phoneme payload keeps any later colons intact.
    let (out, captured) = run_add_rules(&["--rule", "father:ˈfɑː:ðər"]).await;
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sent = captured.lock().unwrap().clone().expect("body");
    let rules = sent["rules"].as_array().unwrap();
    assert_eq!(rules[0]["string_to_replace"], "father");
    // Second colon (ASCII length mark) preserved after the split.
    assert_eq!(rules[0]["phoneme"], "ˈfɑː:ðər");
}

#[tokio::test(flavor = "multi_thread")]
async fn phoneme_rule_trims_surrounding_whitespace() {
    let (out, captured) = run_add_rules(&["--rule", "  run  :  rʌn  "]).await;
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sent = captured.lock().unwrap().clone().expect("body");
    let rules = sent["rules"].as_array().unwrap();
    assert_eq!(rules[0]["string_to_replace"], "run");
    assert_eq!(rules[0]["phoneme"], "rʌn");
}

// ── alias-rule shape ──────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn alias_rule_has_alias_field_not_phoneme() {
    let (out, captured) = run_add_rules(&["--alias-rule", "FBI:F.B.I."]).await;
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sent = captured.lock().unwrap().clone().expect("body");
    let rules = sent["rules"].as_array().unwrap();
    assert_eq!(rules[0]["type"], "alias");
    assert_eq!(rules[0]["string_to_replace"], "FBI");
    assert_eq!(rules[0]["alias"], "F.B.I.");
    // Alias rules don't carry a phoneme or alphabet field.
    assert!(rules[0].get("phoneme").is_none());
    assert!(rules[0].get("alphabet").is_none());
}

// ── mixed rules in one invocation ─────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn phoneme_and_alias_rules_can_mix_in_one_call() {
    let (out, captured) = run_add_rules(&[
        "--rule",
        "tomato:təˈmɑːtoʊ",
        "--alias-rule",
        "SCUBA:scoo-buh",
        "--rule",
        "potato:pəˈteɪtoʊ",
    ])
    .await;
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sent = captured.lock().unwrap().clone().expect("body");
    let rules = sent["rules"].as_array().unwrap();
    assert_eq!(rules.len(), 3);
    // Phoneme rules come first (collected in flag order), then aliases.
    assert_eq!(rules[0]["type"], "phoneme");
    assert_eq!(rules[0]["string_to_replace"], "tomato");
    assert_eq!(rules[1]["type"], "phoneme");
    assert_eq!(rules[1]["string_to_replace"], "potato");
    assert_eq!(rules[2]["type"], "alias");
    assert_eq!(rules[2]["string_to_replace"], "SCUBA");
}

// ── error paths ───────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn rule_without_colon_is_invalid_input() {
    // No mock needed: CLI short-circuits at parse_rule.
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dict", "add-rules", "Bad", "--rule", "no-colon-here"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "expected exit 3; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "invalid_input");
    assert!(
        err["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("WORD:PHONEME"),
        "message should mention the expected shape: {:?}",
        err
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rule_with_empty_word_is_invalid_input() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dict", "add-rules", "Bad", "--rule", ":kæt"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "invalid_input");
}

#[tokio::test(flavor = "multi_thread")]
async fn rule_with_empty_phoneme_is_invalid_input() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dict", "add-rules", "Bad", "--rule", "cat:"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
}

#[tokio::test(flavor = "multi_thread")]
async fn add_rules_with_no_rules_is_invalid_input() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dict", "add-rules", "Empty"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "no rules → exit 3; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "invalid_input");
}
