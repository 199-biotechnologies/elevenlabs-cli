//! Verify `dubbing create` builds the correct multipart body when passed:
//!   (a) a `--source-url`, or
//!   (b) a `--file <path>` on disk.
//!
//! We inspect the body the CLI sends by having wiremock record it via a
//! raw body matcher, then assert that the serialized multipart contains
//! the expected form-field names.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

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

fn body_contains_field(req: &Request, field_name: &str) -> bool {
    // Multipart body — cheap substring check is enough; wiremock gives us raw
    // bytes. Matching `name="<field_name>"` avoids false positives from free
    // text inside other fields.
    let needle = format!("name=\"{field_name}\"");
    String::from_utf8_lossy(&req.body).contains(&needle)
}

#[tokio::test(flavor = "multi_thread")]
async fn create_with_source_url_builds_multipart() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/dubbing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "dubbing_id": "dub_url",
            "expected_duration_sec": 1.0
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
            "https://example.com/clip.mp4",
            "--target-lang",
            "fr",
            "--num-speakers",
            "2",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let req = &requests[0];
    assert!(
        body_contains_field(req, "source_url"),
        "multipart body missing source_url field"
    );
    assert!(
        body_contains_field(req, "target_lang"),
        "multipart body missing target_lang field"
    );
    assert!(
        body_contains_field(req, "num_speakers"),
        "multipart body missing num_speakers field"
    );
    assert!(
        !body_contains_field(req, "file"),
        "multipart body should NOT include file when URL is used"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn create_with_file_builds_multipart() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/dubbing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "dubbing_id": "dub_file",
            "expected_duration_sec": 1.0
        })))
        .mount(&mock)
        .await;

    // Prepare a tiny fake source file.
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("clip.mp4");
    std::fs::write(&src, b"FAKEMP4DATA").unwrap();

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dubbing",
            "create",
            "--file",
            src.to_str().unwrap(),
            "--target-lang",
            "de",
            "--dubbing-studio",
            "true",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let req = &requests[0];
    assert!(
        body_contains_field(req, "file"),
        "multipart body missing file field"
    );
    assert!(
        body_contains_field(req, "target_lang"),
        "multipart body missing target_lang"
    );
    assert!(
        body_contains_field(req, "dubbing_studio"),
        "multipart body missing dubbing_studio"
    );
    assert!(
        !body_contains_field(req, "source_url"),
        "multipart body should NOT include source_url when file is used"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn create_without_source_errors() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        // Any base URL; we should never reach HTTP.
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args(["dubbing", "create", "--target-lang", "es"])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(3),
        "expected invalid_input exit 3; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
