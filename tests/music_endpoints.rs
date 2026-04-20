//! Wiremock regressions pinning the new v0.2 music endpoints to their
//! exact HTTP paths. If any of these routes silently drift (e.g. the
//! server adds a trailing slash, or the CLI uses `/v1/music/detail`
//! instead of `/detailed`), wiremock returns 404 and the assertion fails.
//!
//! Also verifies the happy-path JSON envelope shape so downstream agents
//! can rely on the contract.

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

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

// ── music detailed ─────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn music_detailed_uses_v1_music_detailed() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/detailed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "audio_base64": b64(b"FAKEDETAILED"),
            "bpm": 120,
            "time_signature": "4/4",
            "sections": [{"name": "intro", "duration_ms": 2000}],
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("track.mp3");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "music",
            "detailed",
            "cinematic score",
            "--length-ms",
            "15000",
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
    assert_eq!(body["status"], "success");
    assert!(out_path.exists());
}

// ── music stream ───────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn music_stream_uses_v1_music_stream() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/stream"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"STREAMBYTES".to_vec()))
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
            "ambient pad",
            "--length-ms",
            "8000",
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
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["streamed"], true);
    assert!(out_path.exists());
}

// ── music upload ───────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn music_upload_uses_v1_music_upload() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "song_id": "song_abc123",
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_in = tempfile::tempdir().unwrap();
    let in_path = tmp_in.path().join("track.mp3");
    std::fs::write(&in_path, b"FAKEAUDIO").unwrap();

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "music",
            "upload",
            in_path.to_str().unwrap(),
            "--name",
            "my-track",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["song_id"], "song_abc123");
}

// ── music stem-separation ──────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn music_stem_separation_uses_correct_path() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/stem-separation"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "stems": {
                "vocals": b64(b"VOCALSWAV"),
                "drums": b64(b"DRUMSWAV"),
            }
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "music",
            "stem-separation",
            "song_abc123",
            "--output-dir",
            tmp_out.path().to_str().unwrap(),
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
    let stems = body["data"]["stems_written"].as_array().unwrap();
    assert_eq!(stems.len(), 2);
    assert!(tmp_out.path().join("vocals.mp3").exists());
    assert!(tmp_out.path().join("drums.mp3").exists());
}

// ── music video-to-music ───────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn music_video_to_music_uses_correct_path() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/video-to-music"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"SCOREBYTES".to_vec()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_in = tempfile::tempdir().unwrap();
    let in_path = tmp_in.path().join("clip.mp4");
    std::fs::write(&in_path, b"FAKEVIDEO").unwrap();
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("score.mp3");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "music",
            "video-to-music",
            in_path.to_str().unwrap(),
            "--description",
            "tense thriller",
            "--tag",
            "cinematic",
            "--tag",
            "dark",
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
    assert_eq!(body["status"], "success");
    assert!(out_path.exists());
}
