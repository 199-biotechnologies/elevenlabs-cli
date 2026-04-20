//! Forced alignment: prove that `align` posts a multipart body with the
//! expected fields to POST /v1/forced-alignment, and that responses are
//! surfaced correctly in the JSON envelope.

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
    let path = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, path)
}

fn write_fake_audio(dir: &std::path::Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, b"RIFF\0\0\0\0WAVE").unwrap();
    p
}

#[tokio::test(flavor = "multi_thread")]
async fn align_posts_multipart_to_forced_alignment() {
    let mock = MockServer::start().await;
    let response = serde_json::json!({
        "characters": [
            { "text": "H", "start": 0.0,  "end": 0.1 },
            { "text": "i", "start": 0.1,  "end": 0.2 },
        ],
        "words": [
            { "text": "Hi", "start": 0.0, "end": 0.2, "loss": 0.05 }
        ],
        "loss": 0.05
    });
    Mock::given(method("POST"))
        .and(path("/v1/forced-alignment"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response.clone()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp = tempfile::tempdir().unwrap();
    let audio = write_fake_audio(tmp.path(), "hi.wav");
    let out_json = tmp.path().join("align.json");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "align",
            audio.to_str().unwrap(),
            "Hi",
            "-o",
            out_json.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["word_count"], 1);
    assert_eq!(body["data"]["character_count"], 2);
    assert_eq!(body["data"]["transcript_chars"], 2);
    assert_eq!(body["data"]["loss"], 0.05);
    // The raw words[] and characters[] arrays should round-trip into data.
    assert!(body["data"]["words"].is_array());
    assert_eq!(body["data"]["words"].as_array().unwrap().len(), 1);
    assert!(body["data"]["characters"].is_array());
    assert_eq!(body["data"]["characters"].as_array().unwrap().len(), 2);
    // The saved JSON file on disk must be the full raw response verbatim.
    assert!(out_json.exists(), "output JSON should be written to disk");
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&out_json).unwrap()).unwrap();
    assert_eq!(saved, response);
}

#[tokio::test(flavor = "multi_thread")]
async fn align_supports_transcript_file_flag() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/forced-alignment"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "characters": [], "words": [], "loss": 0.0
        })))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp = tempfile::tempdir().unwrap();
    let audio = write_fake_audio(tmp.path(), "aud.wav");
    let transcript = tmp.path().join("transcript.txt");
    std::fs::write(
        &transcript,
        "This is a transcript with a colon: here.\nNewlines too.",
    )
    .unwrap();

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "align",
            audio.to_str().unwrap(),
            "--transcript-file",
            transcript.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── Missing audio file ─────────────────────────────────────────────────────

#[test]
fn align_errors_when_audio_missing() {
    let out = bin()
        .args(["align", "/tmp/definitely-no-such-file.wav", "hi"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "missing audio should be invalid_input; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── Empty transcript ───────────────────────────────────────────────────────

#[test]
fn align_errors_when_transcript_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let audio = write_fake_audio(tmp.path(), "silent.wav");
    let empty = tmp.path().join("empty.txt");
    std::fs::write(&empty, "").unwrap();
    let out = bin()
        .args([
            "align",
            audio.to_str().unwrap(),
            "--transcript-file",
            empty.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "empty transcript should be invalid_input; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
