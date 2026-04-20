//! A "restricted" API key — one that can list voices but can't read user
//! info — should be flagged as `warn` on the `api_key_scope` check, not
//! as a hard failure. This is a common ElevenLabs key configuration for
//! agents that only need TTS / voice listing.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn bin() -> AssertCmd {
    AssertCmd::cargo_bin("elevenlabs").unwrap()
}

fn temp_config(api_key: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, path)
}

fn find_check<'a>(report: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    let checks = report["data"]["checks"].as_array().expect("checks array");
    checks
        .iter()
        .find(|c| c["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("check {name} missing; report={report}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn restricted_key_warns_not_fails() {
    let mock = MockServer::start().await;

    // /v1/user → 401 (key lacks user_read scope)
    Mock::given(method("GET"))
        .and(path("/v1/user"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "detail": { "status": "permission_denied", "message": "missing scope" }
        })))
        .mount(&mock)
        .await;

    // /v1/voices → 200 with an empty list
    Mock::given(method("GET"))
        .and(path("/v1/voices"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "voices": [] })))
        .mount(&mock)
        .await;

    let (_dir, path) = temp_config("sk_restricted_keyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "doctor", "--skip", "ffmpeg",
            // Network check hits the base URL; the mock accepts HEAD on `/`.
            // Skip it to avoid relying on wiremock's default 404 behaviour here.
            "--skip", "network",
        ])
        .output()
        .unwrap();

    // Warn-only → exit 0.
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0 on warn-only; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["status"], "success");

    let scope = find_check(&report, "api_key_scope");
    assert_eq!(
        scope["status"], "warn",
        "restricted key should warn; got {scope}"
    );
    let detail = scope["detail"].as_str().unwrap();
    assert!(
        detail.contains("restricted") || detail.contains("user_read"),
        "scope warn detail should flag restriction; got: {detail}"
    );

    let summary = &report["data"]["summary"];
    assert!(summary["warn"].as_u64().unwrap() >= 1);
    assert_eq!(summary["fail"].as_u64().unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn fully_unauthorized_key_fails() {
    let mock = MockServer::start().await;

    // Both probes return 401 → the key is fully rejected, not restricted.
    Mock::given(method("GET"))
        .and(path("/v1/user"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "detail": { "status": "invalid_key", "message": "bad key" }
        })))
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/voices"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "detail": { "status": "invalid_key", "message": "bad key" }
        })))
        .mount(&mock)
        .await;

    let (_dir, path) = temp_config("sk_bogus_keyyyyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["doctor", "--skip", "ffmpeg", "--skip", "network"])
        .output()
        .unwrap();

    // Fail → exit 2.
    assert_eq!(
        out.status.code(),
        Some(2),
        "expected exit 2 on fail; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let scope = find_check(&report, "api_key_scope");
    assert_eq!(scope["status"], "fail");
}
