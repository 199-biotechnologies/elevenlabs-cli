//! `elevenlabs doctor` must detect the env-var-shadowing case: when
//! `ELEVENLABS_API_KEY` is set to a different value than the saved
//! `config.toml`. Since v0.1.6 the file wins, so this is informational,
//! but the diagnostic has to surface it (regression of the v0.1.5 bug
//! class where users wondered why their `config init` "didn't work").
//!
//! We skip the API-scope and network checks — they would make real
//! outbound HTTP calls. The env-shadow logic is purely local.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;

fn bin() -> AssertCmd {
    AssertCmd::cargo_bin("elevenlabs").unwrap()
}

fn temp_config(api_key: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, path)
}

fn find_check<'a>(report: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    let checks = report["data"]["checks"].as_array().expect("checks array");
    checks
        .iter()
        .find(|c| c["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("check {name} missing; report={report}"))
}

#[test]
fn env_shadow_warn_when_env_differs_from_file() {
    let (_dir, path) = temp_config("file_key_aaaaaaaaaaaa");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env("ELEVENLABS_API_KEY", "env_key_zzzzzzzzzzzzz")
        .args([
            "doctor",
            "--skip",
            "api_key_scope",
            "--skip",
            "network",
            "--skip",
            "ffmpeg",
        ])
        .output()
        .unwrap();

    // Should still exit 0 (warn only — no fail).
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0 on warn-only; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["status"], "success");

    let env_shadow = find_check(&report, "env_shadow");
    assert_eq!(
        env_shadow["status"], "warn",
        "env_shadow must warn on mismatch; got {env_shadow}"
    );
    let detail = env_shadow["detail"].as_str().unwrap();
    assert!(
        detail.contains("mismatch") || detail.contains("since v0.1.6"),
        "env_shadow detail should explain the shadow; got: {detail}"
    );
    assert!(
        !env_shadow["suggestion"].as_str().unwrap().is_empty(),
        "env_shadow warn must carry a suggestion"
    );

    let summary = &report["data"]["summary"];
    assert!(summary["warn"].as_u64().unwrap() >= 1);
    assert_eq!(summary["fail"].as_u64().unwrap(), 0);
}

#[test]
fn env_shadow_pass_when_env_matches_file() {
    let (_dir, path) = temp_config("same_key_xxxxxxxxxxxx");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env("ELEVENLABS_API_KEY", "same_key_xxxxxxxxxxxx")
        .args([
            "doctor",
            "--skip",
            "api_key_scope",
            "--skip",
            "network",
            "--skip",
            "ffmpeg",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let env_shadow = find_check(&report, "env_shadow");
    assert_eq!(
        env_shadow["status"], "pass",
        "env_shadow should pass when env == file; got {env_shadow}"
    );
}

#[test]
fn env_shadow_pass_when_only_env_set() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml"); // not created
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env("ELEVENLABS_API_KEY", "env_only_key_yyyyyyyy")
        .args([
            "doctor",
            "--skip",
            "api_key_scope",
            "--skip",
            "network",
            "--skip",
            "ffmpeg",
        ])
        .output()
        .unwrap();

    // Config file is missing → warn; API key present (via env) → pass;
    // env_shadow → pass. Overall warn-only, exit 0.
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let env_shadow = find_check(&report, "env_shadow");
    assert_eq!(env_shadow["status"], "pass");
}
