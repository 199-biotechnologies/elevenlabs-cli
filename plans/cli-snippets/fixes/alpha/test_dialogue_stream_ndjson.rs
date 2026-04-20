//! `dialogue --stream` and `dialogue --stream --with-timestamps` expect
//! newline-delimited JSON responses, one `{audio_base64, [alignment]}`
//! object per line. Earlier versions of the CLI buffered the response as
//! a single JSON object, which meant the audio was never decoded
//! correctly when the server emitted multiple chunks. This test pins the
//! NDJSON path: feed a 3-line NDJSON response through wiremock and verify
//! the CLI writes the expected raw-audio bytes (stream case) and a
//! companion JSONL timestamps file with one line per chunk
//! (stream+with-timestamps case).

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

// Build a 3-line NDJSON body where each line carries a distinct audio
// payload. Payloads are intentionally short so we can assert on the exact
// concatenation downstream.
fn build_ndjson_body(with_timestamps: bool) -> Vec<u8> {
    let chunk1 = if with_timestamps {
        format!(
            r#"{{"audio_base64":"{}","alignment":{{"chars":["H"],"char_start_times_ms":[0],"char_durations_ms":[50]}}}}"#,
            b64(b"CHUNK1")
        )
    } else {
        format!(r#"{{"audio_base64":"{}"}}"#, b64(b"CHUNK1"))
    };
    let chunk2 = if with_timestamps {
        format!(
            r#"{{"audio_base64":"{}","alignment":{{"chars":["i"],"char_start_times_ms":[50],"char_durations_ms":[60]}}}}"#,
            b64(b"CHUNK2_LONGER")
        )
    } else {
        format!(r#"{{"audio_base64":"{}"}}"#, b64(b"CHUNK2_LONGER"))
    };
    let chunk3 = if with_timestamps {
        format!(
            r#"{{"audio_base64":"{}","alignment":{{"chars":["!"],"char_start_times_ms":[110],"char_durations_ms":[10]}}}}"#,
            b64(b"END")
        )
    } else {
        format!(r#"{{"audio_base64":"{}"}}"#, b64(b"END"))
    };
    // Three NDJSON lines — the last one terminated with \n, which the
    // parser must also handle (some servers emit a trailing newline).
    format!("{chunk1}\n{chunk2}\n{chunk3}\n").into_bytes()
}

#[tokio::test(flavor = "multi_thread")]
async fn stream_ndjson_decodes_audio_chunks() {
    let mock = MockServer::start().await;
    let body = build_ndjson_body(false);
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue/stream"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("dialogue.mp3");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dialogue",
            "--stream",
            "Alice:v_alice:Hi",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stream should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let wrote = std::fs::read(&out_path).unwrap();
    // Expect CHUNK1 + CHUNK2_LONGER + END concatenated verbatim.
    let mut expected = Vec::new();
    expected.extend_from_slice(b"CHUNK1");
    expected.extend_from_slice(b"CHUNK2_LONGER");
    expected.extend_from_slice(b"END");
    assert_eq!(
        wrote, expected,
        "audio file should be the concatenation of every chunk's decoded payload"
    );

    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["endpoint"], "stream");
    assert_eq!(body["data"]["bytes_written"], expected.len());
}

#[tokio::test(flavor = "multi_thread")]
async fn stream_with_timestamps_ndjson_writes_alignment_jsonl() {
    let mock = MockServer::start().await;
    let body = build_ndjson_body(true);
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue/stream/with-timestamps"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join("dialogue.mp3");
    let ts_path = tmp_out.path().join("dialogue.timings.jsonl");

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dialogue",
            "--stream",
            "--with-timestamps",
            "Alice:v_alice:Hi",
            "-o",
            out_path.to_str().unwrap(),
            "--save-timestamps",
            ts_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stream+with-timestamps should succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Audio — same as the plain-stream test: exact concatenation.
    let wrote = std::fs::read(&out_path).unwrap();
    let mut expected = Vec::new();
    expected.extend_from_slice(b"CHUNK1");
    expected.extend_from_slice(b"CHUNK2_LONGER");
    expected.extend_from_slice(b"END");
    assert_eq!(wrote, expected);

    // Timestamps JSONL — one line per chunk, with `audio_base64` stripped
    // so the file stays diffable / small. Deserialise each line to confirm
    // the alignment payload survived the round-trip.
    let ts = std::fs::read_to_string(&ts_path).unwrap();
    let lines: Vec<&str> = ts.lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "expected one JSONL line per NDJSON chunk; got {}: {ts}",
        lines.len()
    );
    for (i, line) in lines.iter().enumerate() {
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line {i} is not valid JSON ({e}): {line}"));
        assert!(
            v.get("audio_base64").is_none(),
            "audio_base64 must be stripped from the timestamps file; line {i}: {line}"
        );
        assert!(
            v.get("alignment").is_some(),
            "alignment payload must be preserved; line {i}: {line}"
        );
    }
}
