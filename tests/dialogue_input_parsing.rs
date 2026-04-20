//! Input-shape parsing for `dialogue`: both a JSON file path and inline
//! colon-delimited `label:voice_id:text` triples must work interchangeably,
//! with user-friendly errors when inputs are malformed.

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

// ── JSON file input ────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dialogue_accepts_json_file_input() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"X".to_vec()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp = tempfile::tempdir().unwrap();
    let json_in = tmp.path().join("lines.json");
    let contents = serde_json::json!([
        { "text": "First line", "voice_id": "v_alice" },
        { "text": "Second line", "voice_id": "v_bob" },
        { "text": "Third",       "voice_id": "v_alice" },
    ]);
    std::fs::write(&json_in, contents.to_string()).unwrap();

    let out_path = tmp.path().join("out.mp3");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dialogue",
            "--input",
            json_in.to_str().unwrap(),
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["inputs"], 3);
    assert_eq!(body["data"]["unique_voices"], 2);
}

// ── Implicit JSON detection via .json extension ────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dialogue_detects_json_file_via_extension() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"X".to_vec()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp = tempfile::tempdir().unwrap();
    let json_in = tmp.path().join("dlg.json");
    std::fs::write(
        &json_in,
        serde_json::json!([{ "text": "Hi", "voice_id": "v_a" }]).to_string(),
    )
    .unwrap();

    let out_path = tmp.path().join("out.mp3");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dialogue",
            json_in.to_str().unwrap(),
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── Triples: basic ─────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dialogue_parses_colon_triples() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"X".to_vec()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("out.mp3");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dialogue",
            "Alice:v_alice:Hello, world",
            // Text containing colons (URL-like): splitn(3) preserves the rest.
            "Bob:v_bob:Visit https://example.com now",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["inputs"], 2);
    assert_eq!(body["data"]["unique_voices"], 2);
}

// ── Triples: malformed input gives exit 3 with a helpful message ───────────

#[test]
fn dialogue_rejects_malformed_triple() {
    let out = bin()
        .args(["dialogue", "Alice_has_no_colons_at_all"])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(3),
        "malformed triple should be invalid_input; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── Enforced: at most 10 distinct voice IDs ────────────────────────────────

#[test]
fn dialogue_rejects_more_than_ten_distinct_voices() {
    let mut args = vec!["dialogue".to_string()];
    for i in 0..11 {
        args.push(format!("S{i}:v_{i}:line {i}"));
    }
    let out = bin().args(&args).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        ">10 voices should be rejected; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── Missing inputs: `dialogue` with no positionals and no --input ──────────

#[test]
fn dialogue_requires_inputs() {
    let out = bin().args(["dialogue"]).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "no inputs should error; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
