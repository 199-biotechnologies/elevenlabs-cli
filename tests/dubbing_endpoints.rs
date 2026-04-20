//! Wiremock coverage for the dubbing command family. Each test asserts the
//! CLI hits the exact HTTP path + method advertised in AGENTS.md — guarding
//! against regressions where someone "guesses" an API path without checking
//! the Python SDK.
//!
//! Endpoints covered:
//!   * POST   /v1/dubbing
//!   * GET    /v1/dubbing
//!   * GET    /v1/dubbing/{id}
//!   * DELETE /v1/dubbing/{id}
//!   * GET    /v1/dubbing/{id}/audio/{lang}
//!   * POST   /v1/dubbing/resource/{id}/transcribe
//!   * POST   /v1/dubbing/resource/{id}/translate
//!   * POST   /v1/dubbing/resource/{id}/dub
//!   * POST   /v1/dubbing/resource/{id}/render/{lang}
//!   * POST   /v1/dubbing/resource/{id}/migrate-segments

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
    let p = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&p).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, p)
}

// ── POST /v1/dubbing with URL input ────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn create_with_source_url_posts_to_v1_dubbing() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/dubbing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "dubbing_id": "dub_abc",
            "expected_duration_sec": 12.3
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dubbing",
            "create",
            "--source-url",
            "https://example.com/video.mp4",
            "--target-lang",
            "es",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["dubbing_id"], "dub_abc");
}

// ── GET /v1/dubbing ────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn list_gets_v1_dubbing() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/dubbing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "dubs": []
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dubbing", "list"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
}

// ── GET /v1/dubbing/{id} ───────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn show_gets_v1_dubbing_id() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/dubbing/dub_abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "dubbing_id": "dub_abc",
            "status": "dubbed"
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dubbing", "show", "dub_abc"])
        .output()
        .unwrap();

    assert!(out.status.success());
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["dubbing_id"], "dub_abc");
}

// ── DELETE /v1/dubbing/{id} with --yes ─────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn delete_hits_delete_method() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/dubbing/dub_abc"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dubbing", "delete", "dub_abc", "--yes"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected DELETE to succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["deleted"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_without_yes_refuses() {
    // No mock wired — command should never reach the network.
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        // Point at an unroutable base so a regression surfaces as a
        // transient error rather than passing silently.
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dubbing", "delete", "dub_abc"])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(3),
        "expected invalid_input exit 3 without --yes; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── GET /v1/dubbing/{id}/audio/{lang} ──────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn get_audio_downloads_bytes() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/dubbing/dub_abc/audio/es"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"FAKEMP4".to_vec()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("dub.mp4");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dubbing",
            "get-audio",
            "dub_abc",
            "es",
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
    assert_eq!(std::fs::read(&out_path).unwrap(), b"FAKEMP4");
}

// ── Resource endpoints ─────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn resource_transcribe_posts_correct_path() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/dubbing/resource/dub_abc/transcribe"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dubbing", "resource", "transcribe", "dub_abc"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn resource_translate_posts_correct_path() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/dubbing/resource/dub_abc/translate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dubbing", "resource", "translate", "dub_abc"])
        .output()
        .unwrap();

    assert!(out.status.success());
}

#[tokio::test(flavor = "multi_thread")]
async fn resource_dub_posts_correct_path() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/dubbing/resource/dub_abc/dub"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dubbing", "resource", "dub", "dub_abc"])
        .output()
        .unwrap();

    assert!(out.status.success());
}

#[tokio::test(flavor = "multi_thread")]
async fn resource_render_posts_correct_path() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/dubbing/resource/dub_abc/render/es"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dubbing", "resource", "render", "dub_abc", "es"])
        .output()
        .unwrap();

    assert!(out.status.success());
}

#[tokio::test(flavor = "multi_thread")]
async fn resource_migrate_posts_correct_path() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/dubbing/resource/dub_abc/migrate-segments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dubbing", "resource", "migrate-segments", "dub_abc"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
