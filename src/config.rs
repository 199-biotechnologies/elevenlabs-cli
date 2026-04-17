//! Config loading. For the API key specifically, the resolution order is:
//!
//!   1. `--api-key` flag on a command (if present)
//!   2. `api_key` in config.toml                  (EXPLICIT: user ran `config init`)
//!   3. `ELEVENLABS_API_KEY` env var              (AMBIENT: shell / .env files)
//!   4. (not set → `AuthMissing`)
//!
//! As of v0.1.6 the saved config file wins over the env var. Previously the
//! env var won, which caused a silent-auth-failure footgun: a stale/rotated
//! ELEVENLABS_API_KEY left exported by another project would override the
//! key the user just ran `elevenlabs config init` to save, producing only
//! "Invalid API key" with no hint about the real cause.
//!
//! CI / containerised setups are unaffected — they don't ship a config.toml,
//! so the env var is still picked up as the only available source.
//!
//! Escape hatches when you need the env var to take precedence despite a
//! saved config:
//!   - `env -u ELEVENLABS_API_KEY <cmd>` — drop env for one command, but
//!     actually this doesn't apply anymore — file wins anyway.
//!   - `elevenlabs config set api_key <new>` — overwrite the saved value.
//!   - Delete the `api_key` line from config.toml.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// ElevenLabs API key (falls back to ELEVENLABS_API_KEY env var)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Per-command defaults
    #[serde(default)]
    pub defaults: Defaults,

    /// Self-update settings
    #[serde(default)]
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Defaults {
    /// Default voice ID for TTS
    #[serde(default)]
    pub voice_id: Option<String>,

    /// Default model ID for TTS
    #[serde(default)]
    pub model_id: Option<String>,

    /// Default output format
    #[serde(default)]
    pub output_format: Option<String>,

    /// Default output directory for generated files
    #[serde(default)]
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub enabled: bool,
    pub owner: String,
    pub repo: String,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            owner: "199-biotechnologies".into(),
            repo: "elevenlabs-cli".into(),
        }
    }
}

/// Where the effective API key came from. Surfaced in `config show` and in
/// auth-error suggestions so users know which source to edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthSource {
    /// ELEVENLABS_API_KEY environment variable.
    Env,
    /// `api_key` in config.toml.
    File,
    /// No key available anywhere.
    None,
}

impl AuthSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Env => "ELEVENLABS_API_KEY env var",
            Self::File => "config file",
            Self::None => "(unset)",
        }
    }
}

/// Snapshot of every API-key source, for diagnostic output. The CLI consults
/// this when the user runs `config show`, when `config init` saves a new key,
/// and when an auth call fails — so the "env var silently shadows the file"
/// case gets surfaced instead of producing a generic "invalid key" error.
#[derive(Debug, Clone)]
pub struct AuthKeyState {
    /// Raw value of `ELEVENLABS_API_KEY` (trimmed), if set and non-empty.
    pub env_key: Option<String>,
    /// Raw `api_key` from config.toml (trimmed), if present and non-empty.
    pub file_key: Option<String>,
}

impl AuthKeyState {
    /// Read the raw state of both sources. Both values are reported
    /// independently of precedence so callers can diagnose "both set but
    /// only one is used" situations.
    pub fn snapshot(file_key_from_config: Option<&str>) -> Self {
        let env_key = std::env::var("ELEVENLABS_API_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        // Always prefer re-reading the on-disk TOML so env-var overlays in
        // `config::load()` don't masquerade as a file-sourced value.
        let file_key = read_file_api_key().or_else(|| file_key_from_config.map(String::from));
        Self {
            env_key,
            file_key: file_key.filter(|s| !s.trim().is_empty()),
        }
    }

    /// Which source is being used for auth right now. File wins when both
    /// are set — see the module docstring for why.
    pub fn effective_source(&self) -> AuthSource {
        if self.file_key.is_some() {
            AuthSource::File
        } else if self.env_key.is_some() {
            AuthSource::Env
        } else {
            AuthSource::None
        }
    }

    /// The value that actually ships on the wire (file wins over env).
    pub fn effective_key(&self) -> Option<&str> {
        self.file_key.as_deref().or(self.env_key.as_deref())
    }

    /// True iff the env var is set to a value DIFFERENT from the saved
    /// config file key, so it's being ignored. Callers surface this so users
    /// aren't surprised by "I exported ELEVENLABS_API_KEY=X but X wasn't used".
    pub fn env_ignored_by_file(&self) -> bool {
        matches!(
            (&self.env_key, &self.file_key),
            (Some(e), Some(f)) if e.trim() != f.trim()
        )
    }
}

/// Parse just the `api_key` field from the on-disk TOML config. Unlike
/// `load()` this does NOT fold in the env var — we need the file value
/// independently to detect the env-shadow case.
fn read_file_api_key() -> Option<String> {
    let path = config_path();
    let text = std::fs::read_to_string(&path).ok()?;
    let parsed: toml::Value = toml::from_str(&text).ok()?;
    parsed
        .get("api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

impl AppConfig {
    /// Resolve the API key per the v0.1.6 precedence ladder:
    ///   1. config.toml `api_key` (the value the user explicitly saved)
    ///   2. `ELEVENLABS_API_KEY` env var (ambient fallback)
    ///
    /// See the module docstring for the rationale.
    pub fn resolve_api_key(&self) -> Option<String> {
        // 1. Saved config file wins. `self.api_key` was populated from the
        //    TOML by `load()`; the env overlay is intentionally not applied
        //    to this field anymore.
        if let Some(k) = self.api_key.as_ref() {
            let trimmed = k.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        // 2. Env var as a fallback — used by CI / ephemeral containers that
        //    never run `config init`.
        if let Ok(k) = std::env::var("ELEVENLABS_API_KEY") {
            let trimmed = k.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
        None
    }

    /// Build a full snapshot of where keys are and which one wins.
    pub fn auth_key_state(&self) -> AuthKeyState {
        AuthKeyState::snapshot(self.api_key.as_deref())
    }

    /// Voice ID to use if none specified. Falls back to a built-in default.
    pub fn default_voice_id(&self) -> String {
        self.defaults
            .voice_id
            .clone()
            .unwrap_or_else(|| "cgSgspJ2msm6clMCkdW9".to_string())
    }

    /// Model to use for TTS if none specified.
    pub fn default_model_id(&self) -> String {
        self.defaults
            .model_id
            .clone()
            .unwrap_or_else(|| "eleven_multilingual_v2".to_string())
    }

    pub fn default_output_format(&self) -> String {
        self.defaults
            .output_format
            .clone()
            .unwrap_or_else(|| "mp3_44100_128".to_string())
    }
}

pub fn config_path() -> PathBuf {
    // Allow a full-path override for tests and power users. This is the
    // exact path to the config.toml file, not a directory.
    if let Ok(p) = std::env::var("ELEVENLABS_CLI_CONFIG") {
        let p = p.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    directories::ProjectDirs::from("", "", "elevenlabs-cli")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.toml")
}

pub fn load() -> Result<AppConfig, AppError> {
    use figment::Figment;
    use figment::providers::{Env, Format as _, Serialized, Toml};

    // `api_key` comes from the TOML file only. We still honour
    // `ELEVENLABS_CLI_*` for non-secret defaults (voice_id, model_id, etc.),
    // but never for the API key — `resolve_api_key()` layers
    // `ELEVENLABS_API_KEY` on top as a fallback when the file is empty.
    let base = Figment::from(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file(config_path()))
        .merge(Env::prefixed("ELEVENLABS_CLI_").split("_"));

    let cfg: AppConfig = base
        .extract()
        .map_err(|e| AppError::Config(e.to_string()))?;

    Ok(cfg)
}

/// Atomically write the given config to disk, setting 0600 permissions on
/// Unix before the final rename. The write goes through a sibling temp
/// file so concurrent readers never observe a partially-written config,
/// and the temp file has 0600 the whole time so the secret is never
/// briefly world-readable.
pub fn save(cfg: &AppConfig) -> Result<PathBuf, AppError> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let toml = toml::to_string_pretty(cfg)
        .map_err(|e| AppError::Config(format!("serialising config: {e}")))?;

    // Write to a sibling temp file, then rename.
    let tmp_path = path.with_extension("toml.tmp");

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&tmp_path)?;
        f.write_all(toml.as_bytes())?;
        f.sync_all().ok();
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&tmp_path, toml.as_bytes())?;
    }

    std::fs::rename(&tmp_path, &path).map_err(|e| {
        // On rename failure, try to clean up the temp file so we don't
        // leave a world-readable-ish file with a secret in it.
        let _ = std::fs::remove_file(&tmp_path);
        AppError::Io(e)
    })?;

    Ok(path)
}

/// Mask a secret for display: shows prefix and suffix only.
pub fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        return "(not set)".to_string();
    }
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        let prefix: String = chars[..2.min(chars.len())].iter().collect();
        format!("{prefix}***")
    } else {
        let prefix: String = chars[..6].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{prefix}...{suffix}")
    }
}
