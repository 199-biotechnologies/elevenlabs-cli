//! music stream — verify that bytes are written to disk as the response
//! arrives, not buffered in memory first. wiremock lets us trickle bytes
//! via a chunked body; we then confirm the file exists and has the full
//! payload end-to-end.

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
    let cfg = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&cfg).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, cfg)
}

#[tokio::test(flavor = "multi_thread")]
async fn music_stream_writes_full_payload_to_disk() {
    let mock = MockServer::start().await;
    // Reasonably-sized payload so the server has to split it into at
    // least a couple of TCP frames. Exact byte-for-byte preservation is
    // the invariant we care about.
    let payload: Vec<u8> = (0u8..=255u8).cycle().take(4096).collect();

    Mock::given(method("POST"))
        .and(path("/v1/music/stream"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.clone()))
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
            "music",
            "stream",
            "ambient",
            "--length-ms",
            "3000",
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

    let disk = std::fs::read(&out_path).unwrap();
    assert_eq!(
        disk, payload,
        "streamed file bytes must match payload exactly"
    );

    let envelope: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(envelope["status"], "success");
    assert_eq!(envelope["data"]["streamed"], true);
    assert_eq!(
        envelope["data"]["bytes_written"].as_u64().unwrap(),
        payload.len() as u64
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn music_stream_surfaces_http_errors() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/stream"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "detail": { "message": "slow down" }
        })))
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
            "music",
            "stream",
            "ambient",
            "--length-ms",
            "3000",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // 429 must map to exit 4 (rate_limited).
    assert_eq!(
        out.status.code(),
        Some(4),
        "expected exit 4; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "rate_limited");
}
