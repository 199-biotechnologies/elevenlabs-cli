//! `agents add-knowledge` — create a knowledge base document AND splice its
//! id into the agent's `conversation_config.agent.prompt.knowledge_base`.
//!
//! Previously the command created a KB document but never attached it to
//! the target agent — the `agent_id` arg was effectively unused. The fix
//! POSTs the doc, then GETs the agent config, appends the new doc entry
//! under `conversation_config.agent.prompt.knowledge_base`, and PATCHes the
//! updated config back. If the PATCH fails after the doc was created, we
//! surface the doc id in the error so the user can retry the attach step
//! via `agents update --patch <json>` instead of recreating the doc.

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

/// Concrete source of the knowledge-base payload. Moves the three
/// mutually-exclusive flags (`--url` / `--file` / `--text`) into one
/// exhaustive enum so dispatch below is a total `match` — no `unwrap()`
/// needed to convince the compiler exactly one flag is set.
enum KbSource {
    Url(String),
    File(String),
    Text(String),
}

pub async fn add(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    name: String,
    url: Option<String>,
    file: Option<String>,
    text: Option<String>,
) -> Result<(), AppError> {
    let source = match (url, file, text) {
        (Some(u), None, None) => KbSource::Url(u),
        (None, Some(f), None) => KbSource::File(f),
        (None, None, Some(t)) => KbSource::Text(t),
        (None, None, None) => {
            return Err(AppError::bad_input_with(
                "provide one of --url, --file, or --text",
                format!(
                    "elevenlabs agents add-knowledge {agent_id} {name} --file <path>  \
                     (or --url <https://…>, or --text \"<inline content>\")"
                ),
            ));
        }
        _ => {
            return Err(AppError::bad_input_with(
                "provide only one of --url, --file, or --text",
                "pick exactly one source: --file <path> OR --url <https://…> OR --text \"<inline>\"",
            ));
        }
    };

    // ── Step 1: create the KB document ────────────────────────────────────
    let (doc, doc_type) = match source {
        KbSource::Url(u) => {
            let body = serde_json::json!({ "name": name, "url": u });
            let v: serde_json::Value = client
                .post_json("/v1/convai/knowledge-base/url", &body)
                .await?;
            (v, "url")
        }
        KbSource::Text(t) => {
            let body = serde_json::json!({ "name": name, "text": t });
            let v: serde_json::Value = client
                .post_json("/v1/convai/knowledge-base/text", &body)
                .await?;
            (v, "text")
        }
        KbSource::File(f) => {
            let path = Path::new(&f);
            if !path.exists() {
                return Err(AppError::InvalidInput {
                    msg: format!("file does not exist: {}", path.display()),
                    suggestion: None,
                });
            }
            let bytes = crate::commands::read_file_bytes(path).await?;
            let filename = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "file".to_string());
            let mime = crate::commands::mime_for_path(path);
            let file_part = reqwest::multipart::Part::bytes(bytes)
                .file_name(filename)
                .mime_str(&mime)
                .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;
            let form = reqwest::multipart::Form::new()
                .text("name", name.clone())
                .part("file", file_part);
            let v: serde_json::Value = client
                .post_multipart_json("/v1/convai/knowledge-base/file", form)
                .await?;
            (v, "file")
        }
    };

    // The knowledge-base create endpoints return `id` and `name`. Prefer
    // server-returned values so we stay source-of-truth-correct even if the
    // server normalises (e.g. trimming whitespace in the name).
    let doc_id = doc
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| AppError::Api {
            status: 200,
            message: format!(
                "KB document response missing 'id' field; raw response: {}",
                redact(&doc)
            ),
        })?
        .to_string();
    let doc_name = doc
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or(&name)
        .to_string();

    // ── Step 2: fetch the current agent config ────────────────────────────
    let agent_path = format!("/v1/convai/agents/{agent_id}");
    let current: serde_json::Value = match client.get_json(&agent_path).await {
        Ok(v) => v,
        Err(e) => return Err(retry_hint(e, &doc_id, "fetch agent")),
    };

    // ── Step 3: build the PATCH body with the new KB entry appended ───────
    let mut kb_list = current
        .get("conversation_config")
        .and_then(|c| c.get("agent"))
        .and_then(|a| a.get("prompt"))
        .and_then(|p| p.get("knowledge_base"))
        .and_then(|k| k.as_array())
        .cloned()
        .unwrap_or_default();
    kb_list.push(serde_json::json!({
        "id": doc_id,
        "type": doc_type,
        "name": doc_name,
        "usage_mode": "auto",
    }));

    let patch = serde_json::json!({
        "conversation_config": {
            "agent": {
                "prompt": {
                    "knowledge_base": kb_list,
                }
            }
        }
    });

    // ── Step 4: PATCH the agent ───────────────────────────────────────────
    let updated: serde_json::Value = match client.patch_json(&agent_path, &patch).await {
        Ok(v) => v,
        Err(e) => return Err(retry_hint(e, &doc_id, "attach to agent")),
    };

    let result = serde_json::json!({
        "agent_id": agent_id,
        "name": doc_name,
        "document": doc,
        "doc_id": doc_id,
        "attached": true,
        "agent": updated,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        let doc_id = r["doc_id"].as_str().unwrap_or("");
        println!(
            "{} added knowledge '{}' to agent {} (doc {})",
            "+".green(),
            r["name"].as_str().unwrap_or("").bold(),
            r["agent_id"].as_str().unwrap_or("").dimmed(),
            doc_id.dimmed()
        );
    });
    Ok(())
}

/// Wrap an error with the freshly-created doc id so the user can retry
/// just the attach step via `agents update --patch <json>` without paying
/// to recreate the KB document.
fn retry_hint(err: AppError, doc_id: &str, stage: &str) -> AppError {
    let hint = format!(
        " — KB doc '{doc_id}' was created but {stage} failed; retry with: \
         elevenlabs agents update <agent_id> --patch <json containing \
         conversation_config.agent.prompt.knowledge_base entry with id '{doc_id}'>"
    );
    match err {
        AppError::Api { status, message } => AppError::Api {
            status,
            message: format!("{message}{hint}"),
        },
        AppError::AuthFailed(m) => AppError::AuthFailed(format!("{m}{hint}")),
        AppError::RateLimited(m) => AppError::RateLimited(format!("{m}{hint}")),
        AppError::Http(m) => AppError::Http(format!("{m}{hint}")),
        AppError::Transient(m) => AppError::Transient(format!("{m}{hint}")),
        AppError::InvalidInput {
            msg: m,
            suggestion: s,
        } => AppError::InvalidInput {
            msg: format!("{m}{hint}"),
            suggestion: s,
        },
        other => other,
    }
}

/// Pretty-print a JSON value for diagnostic error bodies, truncating very
/// large responses so we don't blow up the terminal.
fn redact(v: &serde_json::Value) -> String {
    let s = serde_json::to_string(v).unwrap_or_else(|_| "<unserialisable>".to_string());
    if s.len() > 400 {
        format!("{}…", &s[..400])
    } else {
        s
    }
}
