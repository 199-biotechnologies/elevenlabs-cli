//! music detailed returns a multipart/mixed body with JSON metadata and
//! binary audio. Verified against the Python SDK's `compose_detailed`
//! method. These tests pin the contract:
//!   - the CLI writes the audio part to `--output`
//!   - the CLI writes the JSON metadata to `--save-metadata`
//!     (default: `<output>.metadata.json`)
//!   - the metadata file contains the JSON verbatim (pretty-printed),
//!     NOT wrapped in an `audio_base64` shell.

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

/// Build a minimal multipart/mixed body: JSON metadata part first, then
/// the audio part. Matches what the ElevenLabs server sends per SDK.
fn build_multipart_mixed(
    boundary: &str,
    json_bytes: &[u8],
    audio_bytes: &[u8],
    audio_mime: &str,
) -> Vec<u8> {
    let mut out = Vec::new();
    // Part 1: JSON metadata
    out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    out.extend_from_slice(b"Content-Type: application/json\r\n\r\n");
    out.extend_from_slice(json_bytes);
    out.extend_from_slice(b"\r\n");
    // Part 2: audio
    out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    out.extend_from_slice(format!("Content-Type: {audio_mime}\r\n\r\n").as_bytes());
    out.extend_from_slice(audio_bytes);
    out.extend_from_slice(b"\r\n");
    // Terminator
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    out
}

#[tokio::test(flavor = "multi_thread")]
async fn music_detailed_splits_multipart_audio_and_metadata() {
    let audio_bytes = b"DECODED_AUDIO_CONTENT";
    let metadata = serde_json::json!({
        "bpm": 128,
        "time_signature": "4/4",
        "key": "C major",
        "sections": [
            {"name": "intro", "start_ms": 0, "end_ms": 4000},
            {"name": "drop",  "start_ms": 4000, "end_ms": 12000},
        ],
    });
    let boundary = "elevenlabs-music-boundary-abc123";
    let body = build_multipart_mixed(
        boundary,
        serde_json::to_vec(&metadata).unwrap().as_slice(),
        audio_bytes,
        "audio/mpeg",
    );

    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/detailed"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "content-type",
                    format!("multipart/mixed; boundary={boundary}").as_str(),
                )
                .set_body_bytes(body),
        )
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let audio_path = tmp_out.path().join("track.mp3");
    let metadata_path = tmp_out.path().join("track.json");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "music",
            "detailed",
            "drum and bass",
            "--length-ms",
            "12000",
            "-o",
            audio_path.to_str().unwrap(),
            "--save-metadata",
            metadata_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Audio file must be exactly the binary part — no base64, no JSON shell.
    let audio_disk = std::fs::read(&audio_path).unwrap();
    assert_eq!(
        audio_disk, audio_bytes,
        "audio file must be the raw binary part from the multipart response"
    );

    // Metadata file must contain the JSON fields.
    let meta_disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&metadata_path).unwrap()).unwrap();
    assert_eq!(meta_disk["bpm"], 128);
    assert_eq!(meta_disk["time_signature"], "4/4");
    assert_eq!(meta_disk["key"], "C major");
    assert!(meta_disk["sections"].is_array());
    assert!(
        meta_disk.get("audio_base64").is_none(),
        "metadata JSON must not include an audio_base64 field"
    );

    // Envelope should reference both paths.
    let envelope: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        envelope["data"]["output"].as_str().unwrap(),
        audio_path.to_str().unwrap()
    );
    assert_eq!(
        envelope["data"]["metadata_path"].as_str().unwrap(),
        metadata_path.to_str().unwrap()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn music_detailed_metadata_default_path_next_to_audio() {
    // Without --save-metadata, the companion file should land next to
    // the audio with a `.metadata.json` suffix.
    let metadata = serde_json::json!({ "bpm": 100 });
    let boundary = "b123";
    let body = build_multipart_mixed(
        boundary,
        serde_json::to_vec(&metadata).unwrap().as_slice(),
        b"AUDIO",
        "audio/mpeg",
    );

    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/detailed"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "content-type",
                    format!("multipart/mixed; boundary={boundary}").as_str(),
                )
                .set_body_bytes(body),
        )
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let audio_path = tmp_out.path().join("out.mp3");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "music",
            "detailed",
            "jazz trio",
            "-o",
            audio_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let expected_meta = tmp_out.path().join("out.mp3.metadata.json");
    assert!(
        expected_meta.exists(),
        "default metadata path <audio>.metadata.json was not created at {}",
        expected_meta.display()
    );
}
