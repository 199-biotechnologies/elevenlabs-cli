//! Client-side voice-name → voice_id resolver. Exact > prefix > substring
//! match. Error if no match. Never silently pick the first result — that's
//! the bug that bit us in v0.1.0.
//!
//! Exposed so other command trees (tts.rs, audio.rs) can drop their inline
//! copies and share this one. The follow-up in NOTES.md tracks that refactor.

use crate::client::ElevenLabsClient;
use crate::error::AppError;

/// Resolve a user-supplied voice name to a voice_id. Uses `/v2/voices` so the
/// server-side `search` param actually filters (the v1 endpoint ignores it,
/// which is what caused the v0.1.0 silent-pick regression).
#[allow(dead_code)]
pub async fn resolve_voice_id_by_name(
    client: &ElevenLabsClient,
    name: &str,
) -> Result<String, AppError> {
    let query = [("search", name)];
    let resp: serde_json::Value = client.get_json_with_query("/v2/voices", &query).await?;
    let voices = resp
        .get("voices")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let needle = name.to_lowercase();
    let mut exact: Option<&serde_json::Value> = None;
    let mut prefix: Option<&serde_json::Value> = None;
    let mut substring: Option<&serde_json::Value> = None;

    for v in &voices {
        let Some(vname) = v.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        let lower = vname.to_lowercase();
        if lower == needle {
            exact = Some(v);
            break;
        }
        if prefix.is_none() && lower.starts_with(&needle) {
            prefix = Some(v);
        }
        if substring.is_none() && lower.contains(&needle) {
            substring = Some(v);
        }
    }

    if let Some(v) = exact.or(prefix).or(substring) {
        if let Some(id) = v.get("voice_id").and_then(|n| n.as_str()) {
            return Ok(id.to_string());
        }
    }

    Err(AppError::InvalidInput {
        msg: format!(
            "no voice in your library matches '{name}'. \
         List voices with: elevenlabs voices list"
        ),
        suggestion: None,
    })
}
