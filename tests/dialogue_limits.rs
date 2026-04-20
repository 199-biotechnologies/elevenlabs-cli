//! Client-side guardrails for `elevenlabs dialogue`. The server enforces
//! these same limits, but running them through the API burns quota and
//! returns slow 422s. The CLI pre-validates so bad inputs fail fast with
//! exit 3 (InvalidInput) without a network call.
//!
//! Gemini's v0.2 review flagged that we had no "red-path" coverage for
//! these limits. This module pins them down.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;

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

#[test]
fn too_many_voices_rejected_exit_3() {
    // 11 distinct voice IDs — one above the documented limit of 10.
    let mut args: Vec<String> = vec!["--json".into(), "dialogue".into()];
    for i in 0..11 {
        args.push(format!("Speaker{i}:voice_id_{i}:line {i}"));
    }

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        // Point at a bogus base URL so any accidental network call fails
        // loudly — but the guardrail should fire before we reach the client.
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args(&args)
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(3),
        "11 voices must fail with exit 3 (InvalidInput); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let env: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("error envelope must be valid JSON on stderr");
    assert_eq!(env["status"], "error");
    assert_eq!(env["error"]["code"], "invalid_input");
    let msg = env["error"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("11") && msg.to_lowercase().contains("voice"),
        "error message should mention the 11 voices it got: {msg}"
    );
}

#[test]
fn total_text_over_2000_chars_rejected_exit_3() {
    // One speaker, 2001 characters of text. The CLI caps at ~2000.
    let long_line = "a".repeat(2001);
    let triple = format!("Alice:voice_id_alice:{long_line}");

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args(["--json", "dialogue", &triple])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(3),
        "2001-char dialogue must fail with exit 3 (InvalidInput); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let env: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("error envelope must be valid JSON on stderr");
    assert_eq!(env["error"]["code"], "invalid_input");
    let msg = env["error"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("2001") || msg.to_lowercase().contains("char"),
        "error message should mention the character count: {msg}"
    );
}

#[test]
fn empty_inputs_rejected_exit_3() {
    // Empty JSON array — passes `parse_inputs` but is caught by the
    // `inputs.is_empty()` check in `run`.
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("empty.json");
    std::fs::write(&json_path, b"[]").unwrap();

    let (_cfg_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args(["--json", "dialogue", "--input", json_path.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(3),
        "empty dialogue must fail with exit 3 (InvalidInput); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let env: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("error envelope must be valid JSON on stderr");
    assert_eq!(env["error"]["code"], "invalid_input");
    let msg = env["error"]["message"].as_str().unwrap_or("");
    assert!(
        msg.to_lowercase().contains("no inputs")
            || msg.to_lowercase().contains("empty")
            || msg.to_lowercase().contains("inputs"),
        "error message should mention empty/no inputs: {msg}"
    );
    // The fixer attached a command-specific suggestion — verify it landed.
    let suggestion = env["error"]["suggestion"].as_str().unwrap_or("");
    assert!(
        suggestion.contains("elevenlabs dialogue"),
        "empty-inputs error should carry a per-command suggestion: {suggestion}"
    );
}
