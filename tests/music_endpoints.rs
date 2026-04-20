//! Wiremock regressions pinning the new v0.2 music endpoints to their
//! exact HTTP paths. If any of these routes silently drift (e.g. the
//! server adds a trailing slash, or the CLI uses `/v1/music/detail`
//! instead of `/detailed`), wiremock returns 404 and the assertion
//! fails.
//!
//! Response shapes are deliberately minimal — just enough to exercise
//! the HTTP contract. The shape-level invariants live in the dedicated
//! `music_multipart.rs` / `music_detailed_split.rs` suites.

use assert_cmd::Command as AssertCmd;
use std::io::{Cursor, Write};
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

/// Build a minimal multipart/mixed body with JSON metadata + audio.
/// Shared between this suite and `music_detailed_split.rs`.
fn build_multipart_mixed(boundary: &str, json_bytes: &[u8], audio_bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    out.extend_from_slice(b"Content-Type: application/json\r\n\r\n");
    out.extend_from_slice(json_bytes);
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    out.extend_from_slice(b"Content-Type: audio/mpeg\r\n\r\n");
    out.extend_from_slice(audio_bytes);
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    out
}

/// Build a minimal zip archive in memory containing the named entries.
/// Used to fake the stem-separation response.
fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::<u8>::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, bytes) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }
    buf.into_inner()
}

// ── music detailed ─────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn music_detailed_uses_v1_music_detailed() {
    let metadata = serde_json::json!({
        "bpm": 120,
        "time_signature": "4/4",
        "sections": [{"name": "intro", "duration_ms": 2000}],
    });
    let boundary = "endpoint-test-boundary";
    let body = build_multipart_mixed(
        boundary,
        serde_json::to_vec(&metadata).unwrap().as_slice(),
        b"FAKEDETAILED",
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
        .args(["music", "upload", in_path.to_str().unwrap()])
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
async fn music_stem_separation_uses_correct_path_and_unzips() {
    // Real endpoint returns a ZIP archive; fabricate one with two stems.
    let zip = build_zip(&[("vocals.mp3", b"VOCALS"), ("drums.mp3", b"DRUMS")]);

    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/stem-separation"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/zip")
                .set_body_bytes(zip),
        )
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_in = tempfile::tempdir().unwrap();
    let in_path = tmp_in.path().join("source.mp3");
    std::fs::write(&in_path, b"FAKEAUDIO").unwrap();
    let tmp_out = tempfile::tempdir().unwrap();

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "music",
            "stem-separation",
            in_path.to_str().unwrap(),
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
    assert_eq!(
        std::fs::read(tmp_out.path().join("vocals.mp3")).unwrap(),
        b"VOCALS"
    );
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
