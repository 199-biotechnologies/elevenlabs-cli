//! Happy-path structural assertions for `elevenlabs doctor`. Verifies the
//! JSON envelope shape (`{checks, summary}`), canonical check names, the
//! `--skip` flag, and that warn-only reports still exit 0.
//!
//! We mock `/v1/user` and `/v1/voices` so the API-scope check passes, and
//! skip `ffmpeg` (not every CI host has it) and the network check (the
//! wiremock server only responds on the mocked endpoints).

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

#[tokio::test(flavor = "multi_thread")]
async fn doctor_envelope_shape_and_happy_path() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/user"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "user_id": "u_1", "tier": "creator" })),
        )
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/voices"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "voices": [] })))
        .mount(&mock)
        .await;

    let (_dir, path) = temp_config("sk_happy_keyyyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(["doctor", "--skip", "ffmpeg", "--skip", "network"])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["version"], "1");
    assert_eq!(report["status"], "success");
    let data = &report["data"];
    assert!(data["checks"].is_array());
    assert!(data["summary"].is_object());

    let checks = data["checks"].as_array().unwrap();
    // Four canonical names must be present (the ones we didn't skip).
    let names: Vec<&str> = checks.iter().map(|c| c["name"].as_str().unwrap()).collect();
    for required in [
        "config_file",
        "api_key",
        "env_shadow",
        "api_key_scope",
        "disk_write",
        "output_dir",
    ] {
        assert!(
            names.contains(&required),
            "check {required} missing from report; got {names:?}"
        );
    }
    // Skipped checks must not appear.
    assert!(!names.contains(&"ffmpeg"), "ffmpeg was skipped");
    assert!(!names.contains(&"network"), "network was skipped");

    // Every check has {name, status, detail, suggestion}.
    for c in checks {
        assert!(c["name"].is_string(), "check missing name: {c}");
        assert!(
            ["pass", "warn", "fail"].contains(&c["status"].as_str().unwrap()),
            "check status invalid: {c}"
        );
        assert!(c["detail"].is_string(), "check missing detail: {c}");
        assert!(c["suggestion"].is_string(), "check missing suggestion: {c}");
    }

    let summary = &data["summary"];
    let p = summary["pass"].as_u64().unwrap();
    let w = summary["warn"].as_u64().unwrap();
    let f = summary["fail"].as_u64().unwrap();
    assert_eq!(
        p + w + f,
        checks.len() as u64,
        "summary must cover all checks"
    );
    assert_eq!(f, 0, "happy path must have no failures");

    // api_key_scope should be pass (both endpoints returned 200).
    let scope = checks
        .iter()
        .find(|c| c["name"] == "api_key_scope")
        .unwrap();
    assert_eq!(scope["status"], "pass");
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_respects_skip_flag() {
    let (_dir, path) = temp_config("sk_skip_keyyyyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "doctor",
            "--skip",
            "api_key_scope",
            "--skip",
            "network",
            "--skip",
            "ffmpeg",
        ])
        .output()
        .unwrap();

    // We skipped the only failure sources (scope + network). The remaining
    // checks should all pass → exit 0.
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let checks = report["data"]["checks"].as_array().unwrap();
    let names: Vec<&str> = checks.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert!(!names.contains(&"api_key_scope"));
    assert!(!names.contains(&"network"));
    assert!(!names.contains(&"ffmpeg"));
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_missing_api_key_fails_with_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml"); // not created
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "doctor",
            "--skip",
            "network",
            "--skip",
            "ffmpeg",
            "--skip",
            "api_key_scope",
        ])
        .output()
        .unwrap();

    // api_key check fails (no key anywhere) → exit 2.
    assert_eq!(
        out.status.code(),
        Some(2),
        "expected exit 2; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let checks = report["data"]["checks"].as_array().unwrap();
    let api = checks.iter().find(|c| c["name"] == "api_key").unwrap();
    assert_eq!(api["status"], "fail");
    assert!(!api["suggestion"].as_str().unwrap().is_empty());
}
