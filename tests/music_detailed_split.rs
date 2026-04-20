//! music detailed writes two files: the decoded audio and a companion
//! JSON with everything except the raw audio. This locks in the split
//! contract so downstream tooling can rely on both files being written.

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
    let cfg = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&cfg).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, cfg)
}

#[tokio::test(flavor = "multi_thread")]
async fn music_detailed_splits_audio_and_metadata() {
    let audio_bytes = b"DECODED_AUDIO_CONTENT";
    let audio_b64 = base64::engine::general_purpose::STANDARD.encode(audio_bytes);

    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/detailed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "audio_base64": audio_b64,
            "bpm": 128,
            "time_signature": "4/4",
            "key": "C major",
            "sections": [
                {"name": "intro", "start_ms": 0, "end_ms": 4000},
                {"name": "drop",  "start_ms": 4000, "end_ms": 12000},
            ],
        })))
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

    // Audio file must contain the decoded base64 body, not the base64 text.
    let audio_disk = std::fs::read(&audio_path).unwrap();
    assert_eq!(
        audio_disk, audio_bytes,
        "audio file must be the decoded base64 content"
    );

    // Metadata JSON must be valid and contain the non-audio fields, and
    // MUST NOT contain the `audio_base64` field (which would duplicate
    // the payload and bloat the JSON).
    let meta_disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&metadata_path).unwrap()).unwrap();
    assert_eq!(meta_disk["bpm"], 128);
    assert_eq!(meta_disk["time_signature"], "4/4");
    assert_eq!(meta_disk["key"], "C major");
    assert!(meta_disk["sections"].is_array());
    assert!(
        meta_disk.get("audio_base64").is_none(),
        "metadata JSON must not include the base64 audio blob"
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
    // Without --save-metadata, the companion file should land next to the
    // audio with a `.metadata.json` suffix.
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/detailed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "audio_base64": base64::engine::general_purpose::STANDARD.encode(b"AUDIO"),
            "bpm": 100,
        })))
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
