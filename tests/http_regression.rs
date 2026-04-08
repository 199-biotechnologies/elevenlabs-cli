//! HTTP-mocked regression tests for the 3 silently-regressible bugs
//! fixed between v0.1.0 and v0.1.1:
//!
//!   1. `--voice NAME` used to silently pick the first result when the
//!      server-side search didn't actually match. Must now error with
//!      invalid_input (exit 3) when no voice matches.
//!   2. `music compose` used to hit `/v1/music/compose` (404). Correct
//!      path is `/v1/music`.
//!   3. `music plan` used to hit `/v1/music/plans/compose` (404).
//!      Correct path is `/v1/music/plan`.
//!
//! We also lock in that `--loop` is the real CLI flag name (v0.1.0 had
//! `--looping` because `#[arg(long, name = "loop")]` didn't rename).
//!
//! These tests spin up a local wiremock server, point the CLI at it via
//! ELEVENLABS_API_BASE_URL, and assert on the exact request paths the
//! CLI emits.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;
use wiremock::matchers::{header, method, path};
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

// ── Voice resolver regression ──────────────────────────────────────────────
//
// The mock /v1/voices endpoint returns a fixed list that deliberately does
// NOT contain the name "definitely-not-a-real-voice". The CLI must refuse
// with invalid_input (exit 3), not silently pick the first result.

#[tokio::test(flavor = "multi_thread")]
async fn voice_name_resolver_errors_on_no_match() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .and(header("xi-api-key", "sk_test_keyyyyyyyyy"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": [
                {"voice_id": "v1", "name": "Rachel", "category": "premade"},
                {"voice_id": "v2", "name": "Adam",   "category": "premade"},
            ]
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "tts",
            "hello",
            "--voice",
            "definitely-not-a-real-voice",
            "-o",
            "/tmp/should-not-exist.mp3",
        ])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(3),
        "expected exit 3 (invalid_input), got {:?}. stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "invalid_input");
    assert!(
        err["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("definitely-not-a-real-voice"),
        "error message should mention the bad voice name"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn voice_name_resolver_matches_substring() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": [
                {"voice_id": "v1", "name": "Some Other Voice",  "category": "premade"},
                {"voice_id": "v_rachel_id", "name": "Rachel - Narrator", "category": "premade"},
            ]
        })))
        .mount(&mock)
        .await;
    // TTS then posts to /v1/text-to-speech/v_rachel_id
    Mock::given(method("POST"))
        .and(path("/v1/text-to-speech/v_rachel_id"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"FAKEMP3".to_vec()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("out.mp3");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "tts",
            "hello world",
            "--voice",
            "Rachel",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["voice_id"], "v_rachel_id");
}

// ── Music endpoint path regression ─────────────────────────────────────────
//
// These tests wire up a mock that ONLY matches the correct paths. If the
// CLI regresses to /v1/music/compose or /v1/music/plans/compose, the mock
// won't match and wiremock returns 404 → the assertion fails.

#[tokio::test(flavor = "multi_thread")]
async fn music_compose_uses_v1_music() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music")) // exact — not /v1/music/compose
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"FAKEMUSIC".to_vec()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("music.mp3");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "music",
            "compose",
            "a cheerful jingle",
            "--length-ms",
            "15000",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected POST /v1/music to succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert!(out_path.exists(), "music file should have been written");
}

#[tokio::test(flavor = "multi_thread")]
async fn music_plan_uses_v1_music_plan() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/plan")) // exact — not /v1/music/plans/compose
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sections": [{"name": "intro", "duration_ms": 5000}]
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["music", "plan", "lofi ambient"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected POST /v1/music/plan to succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
}

// ── --loop flag name regression (no mock needed) ──────────────────────────
//
// v0.1.0 had `#[arg(long, name = "loop")]` which clap honoured as
// `--looping` (the field name), not `--loop`. This test asserts the flag
// parses and reaches a real execution path. We use a bogus base URL so
// it fails at HTTP, but importantly NOT at clap parsing.

// ── Secret redaction regression ────────────────────────────────────────────
// If the upstream echoes our API key back in an error body (or a proxy
// rewrites it into the message), the client must scrub it before it hits
// the JSON envelope we print to stderr.

#[tokio::test(flavor = "multi_thread")]
async fn api_error_redacts_api_keys_from_body() {
    let mock = MockServer::start().await;
    // Return a 500 with a body that contains an sk_ key literal.
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "detail": {
                "status": "internal_error",
                "message": "internal error with token sk_abcdefghijklmnop1234"
            }
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["voices", "list"])
        .output()
        .unwrap();

    // Must fail, but the 'sk_' key from the body must be redacted.
    assert_ne!(out.status.code(), Some(0), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("sk_abcdefghijklmnop"),
        "API key leaked into error: {stderr}"
    );
    assert!(
        stderr.contains("sk_***"),
        "expected redaction marker in: {stderr}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn sfx_loop_flag_parses() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/sound-generation"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"FAKESFX".to_vec()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("sfx.mp3");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "sfx",
            "rain on tin",
            "--loop",
            "--duration",
            "5",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "`--loop` should parse and reach HTTP; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["looping"], true);
}
