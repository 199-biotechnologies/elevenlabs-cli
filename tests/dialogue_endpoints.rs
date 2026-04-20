//! Prove that `dialogue` routes to the correct endpoint for each combination
//! of --stream / --with-timestamps.
//!
//! The four variants map as follows (grounded against elevenlabs-python):
//!   - default                    → POST /v1/text-to-dialogue
//!   - --stream                   → POST /v1/text-to-dialogue/stream
//!   - --with-timestamps          → POST /v1/text-to-dialogue/with-timestamps
//!   - --stream --with-timestamps → POST /v1/text-to-dialogue/stream/with-timestamps
//!
//! Regressions in path routing would cause silent 404s against the real API,
//! so we lock in all four with a wiremock that only matches the expected
//! path. Any drift fails the test.

use assert_cmd::Command as AssertCmd;
use base64::Engine as _;
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

fn fake_b64_audio() -> String {
    base64::engine::general_purpose::STANDARD.encode(b"FAKEAUDIO")
}

// ── default (non-streaming, bytes) ─────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dialogue_default_posts_to_root() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"FAKEDIALOGUE".to_vec()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("dialogue.mp3");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dialogue",
            "Alice:v_alice:Hello there",
            "Bob:v_bob:Hi!",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected POST /v1/text-to-dialogue to succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["endpoint"], "convert");
    assert_eq!(body["data"]["inputs"], 2);
    assert_eq!(body["data"]["unique_voices"], 2);
    assert!(out_path.exists());
}

// ── --stream (bytes) ───────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dialogue_stream_posts_to_stream() {
    // Streaming contract per elevenlabs-python: NDJSON with {audio_base64}
    // per line. Each line is base64-decoded and appended to --output.
    let ndjson = format!(
        "{{\"audio_base64\":\"{}\"}}\n{{\"audio_base64\":\"{}\"}}\n",
        fake_b64_audio(),
        fake_b64_audio()
    );
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue/stream"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ndjson))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("stream.mp3");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dialogue",
            "--stream",
            "Alice:v_alice:Streaming",
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
    assert_eq!(body["data"]["endpoint"], "stream");
}

// ── --with-timestamps (JSON) ───────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dialogue_with_timestamps_posts_to_with_timestamps() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue/with-timestamps"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "audio_base64": fake_b64_audio(),
            "alignment": { "characters": [], "character_start_times_seconds": [] }
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("with_ts.mp3");
    let timings_path = tmp_out.path().join("timings.json");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dialogue",
            "--with-timestamps",
            "Alice:v_alice:Line one",
            "-o",
            out_path.to_str().unwrap(),
            "--save-timestamps",
            timings_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["endpoint"], "with-timestamps");
    assert!(out_path.exists(), "audio should be decoded and saved");
    assert!(timings_path.exists(), "alignment JSON should be saved");
    // The persisted alignment file must NOT contain the base64 audio blob.
    let saved = std::fs::read_to_string(&timings_path).unwrap();
    assert!(
        !saved.contains("audio_base64"),
        "alignment dump should have audio_base64 stripped"
    );
}

// ── --stream + --with-timestamps (JSON chunks) ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn dialogue_stream_with_timestamps_posts_to_combined_path() {
    // Streaming+timestamps contract: NDJSON with {audio_base64, alignment}
    // per line. Audio appended to --output, alignment appended to a JSONL
    // companion.
    let ndjson = format!(
        "{{\"audio_base64\":\"{}\",\"alignment\":{{\"characters\":[]}}}}\n\
         {{\"audio_base64\":\"{}\",\"alignment\":{{\"characters\":[]}}}}\n",
        fake_b64_audio(),
        fake_b64_audio()
    );
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue/stream/with-timestamps"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ndjson))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("combined.mp3");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dialogue",
            "--stream",
            "--with-timestamps",
            "Alice:v_alice:Chunk one",
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
    assert_eq!(body["data"]["endpoint"], "stream+with-timestamps");
    assert!(out_path.exists());
}
