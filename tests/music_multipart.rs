//! music upload + video-to-music use multipart bodies. These tests lock
//! in that the CLI sends the expected form-data shape (file part under
//! the right field name, optional text fields attached).
//!
//! Contract per the SDK (verified April 2026):
//!   - /v1/music/upload expects a part named `file`, optional text field
//!     `extract_composition_plan` (bool). NO `name` or
//!     `composition_plan` form fields.
//!   - /v1/music/video-to-music expects the part named `videos` (NOT
//!     `file`). NO `model_id` field.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

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

/// Assert the request is multipart and that the body contains all
/// `expected_tokens`, and that none of the `forbidden_tokens` appear.
struct AssertMultipartContains {
    expected_tokens: Vec<&'static str>,
    forbidden_tokens: Vec<&'static str>,
    response: ResponseTemplate,
}

impl Respond for AssertMultipartContains {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        let ct = req
            .headers
            .get("content-type")
            .map(|v| v.to_str().unwrap_or(""))
            .unwrap_or("");
        assert!(
            ct.starts_with("multipart/form-data"),
            "expected multipart request, got content-type={ct}"
        );
        let body = String::from_utf8_lossy(&req.body);
        for t in &self.expected_tokens {
            assert!(
                body.contains(t),
                "expected multipart body to contain '{t}', full body:\n{body}"
            );
        }
        for t in &self.forbidden_tokens {
            assert!(
                !body.contains(t),
                "multipart body must NOT contain '{t}' (dropped in v0.2), full body:\n{body}"
            );
        }
        self.response.clone()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn music_upload_sends_multipart_with_file_and_extract_flag() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/upload"))
        .respond_with(AssertMultipartContains {
            expected_tokens: vec![
                "name=\"file\"",
                "name=\"extract_composition_plan\"",
                "FAKEAUDIO",
            ],
            // Dropped in v0.2 — these fields don't exist in the SDK.
            forbidden_tokens: vec!["name=\"name\"", "name=\"composition_plan\""],
            response: ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "song_id": "song_123",
            })),
        })
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_in = tempfile::tempdir().unwrap();
    let in_path = tmp_in.path().join("song.mp3");
    std::fs::write(&in_path, b"FAKEAUDIO").unwrap();

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "music",
            "upload",
            in_path.to_str().unwrap(),
            "--extract-composition-plan",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["data"]["song_id"], "song_123");
}

#[tokio::test(flavor = "multi_thread")]
async fn music_video_to_music_sends_multipart_with_videos_part_and_hints() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/video-to-music"))
        .respond_with(AssertMultipartContains {
            // Part name must be `videos` per the SDK. Tags are repeated
            // under the same form field name.
            expected_tokens: vec![
                "name=\"videos\"",
                "name=\"description\"",
                "tense thriller",
                "name=\"tags\"",
                "cinematic",
                "FAKEVIDEO",
            ],
            // `model_id` was removed — this endpoint doesn't accept it.
            forbidden_tokens: vec!["name=\"model_id\""],
            response: ResponseTemplate::new(200).set_body_bytes(b"SCORE".to_vec()),
        })
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_in = tempfile::tempdir().unwrap();
    let in_path = tmp_in.path().join("scene.mp4");
    std::fs::write(&in_path, b"FAKEVIDEO").unwrap();
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("out.mp3");

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
    assert!(out_path.exists());
}
