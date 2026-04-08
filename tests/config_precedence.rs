//! Contract test: env var must win over the config file per the
//! documented precedence ladder (CLI flags > env > config > defaults).

use assert_cmd::Command;
use std::io::Write;

fn bin() -> Command {
    Command::cargo_bin("elevenlabs").unwrap()
}

/// Write a TOML config under a fake HOME so the test doesn't touch real
/// state. Returns the HOME path so the test can set it on Command.
fn fake_home_with_config(api_key: &str) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    // macOS uses ~/Library/Application Support/<app>/, Linux uses
    // ~/.config/<app>/. Write to both so the test runs on either.
    for rel in [
        "Library/Application Support/elevenlabs-cli",
        ".config/elevenlabs-cli",
    ] {
        let dir = home.path().join(rel);
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = std::fs::File::create(dir.join("config.toml")).unwrap();
        writeln!(f, "api_key = \"{api_key}\"").unwrap();
    }
    home
}

fn extract_api_key(stdout: &[u8]) -> String {
    let v: serde_json::Value = serde_json::from_slice(stdout).unwrap();
    v["data"]["api_key"].as_str().unwrap_or("").to_string()
}

#[test]
fn env_var_wins_over_config_file() {
    let home = fake_home_with_config("config_key_xxxxxxxxxxxx");
    let out = bin()
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        .env("ELEVENLABS_API_KEY", "env_key_yyyyyyyyyyyyy")
        .args(["config", "show", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "config show should exit 0");
    let masked = extract_api_key(&out.stdout);
    // Masked format is "env_ke...yyyy" or similar — we only need to know
    // the env-var's distinctive prefix/suffix wins over the config one.
    assert!(
        masked.starts_with("env_ke"),
        "expected env key to win, got masked={masked}"
    );
    assert!(
        !masked.contains("config"),
        "masked should not contain 'config_'"
    );
}

#[test]
fn config_file_wins_over_defaults_when_no_env() {
    let home = fake_home_with_config("config_key_aaaaaaaaaaaa");
    let out = bin()
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        .env_remove("ELEVENLABS_API_KEY")
        .env_remove("ELEVENLABS_CLI_API_KEY")
        .args(["config", "show", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "config show should exit 0");
    let masked = extract_api_key(&out.stdout);
    assert!(
        masked.starts_with("config"),
        "expected config key, got masked={masked}"
    );
}
