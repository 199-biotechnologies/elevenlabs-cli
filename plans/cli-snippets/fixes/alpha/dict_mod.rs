//! Pronunciation dictionaries — list / add-file / add-rules / show / update /
//! download / set-rules / add-rules-to / remove-rules.
//!
//! API reference: https://elevenlabs.io/docs/api-reference/pronunciation-dictionaries
//!
//! ElevenLabs represents a "rule" as one of:
//!
//!   { "string_to_replace": "word", "type": "phoneme", "phoneme": "...", "alphabet": "ipa" }
//!   { "string_to_replace": "word", "type": "alias",   "alias":   "..."                     }
//!
//! At the CLI we expose `--rule word:phoneme` (phoneme rule, IPA alphabet) and
//! `--alias-rule word:alias` (alias rule). Both flags are repeatable so the
//! caller can send many rules in one request. The value is split on the FIRST
//! `:` only, so IPA phonemes containing colons (tone marks, ejectives, etc.)
//! survive intact.

pub mod add_file;
pub mod add_rules;
pub mod add_rules_to;
pub mod download;
pub mod list;
pub mod remove_rules;
pub mod set_rules;
pub mod show;
pub mod update;

use crate::cli::DictAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::Ctx;

pub async fn dispatch(ctx: Ctx, action: DictAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    match action {
        DictAction::List {
            cursor,
            page_size,
            search,
        } => list::run(ctx, &client, cursor, page_size, search).await,

        DictAction::AddFile {
            name,
            file,
            description,
            workspace_access,
        } => add_file::run(ctx, &client, name, file, description, workspace_access).await,

        DictAction::AddRules {
            name,
            description,
            workspace_access,
            rule,
            alias_rule,
        } => {
            add_rules::run(
                ctx,
                &client,
                name,
                description,
                workspace_access,
                rule,
                alias_rule,
            )
            .await
        }

        DictAction::Show { id } => show::run(ctx, &client, &id).await,

        DictAction::Update {
            id,
            name,
            description,
            archive,
        } => update::run(ctx, &client, id, name, description, archive).await,

        DictAction::Download {
            id,
            version,
            output,
        } => download::run(ctx, &client, id, version, output).await,

        DictAction::SetRules {
            id,
            rule,
            alias_rule,
            case_sensitive,
            word_boundaries,
        } => {
            set_rules::run(
                ctx,
                &client,
                id,
                rule,
                alias_rule,
                case_sensitive,
                word_boundaries,
            )
            .await
        }

        DictAction::AddRulesTo {
            id,
            rule,
            alias_rule,
        } => add_rules_to::run(ctx, &client, id, rule, alias_rule).await,

        DictAction::RemoveRules { id, word } => remove_rules::run(ctx, &client, id, word).await,
    }
}

// ── Shared helpers ──────────────────────────────────────────────────────────

/// Parse a `word:phoneme` CLI rule into the API body shape for a phoneme rule.
/// The split happens on the FIRST `:` only so IPA phonemes that embed `:`
/// (length marks, tones, ejectives, …) round-trip untouched. The alphabet is
/// always `ipa` — that is the only alphabet the API accepts right now.
pub fn parse_rule(raw: &str) -> Result<serde_json::Value, AppError> {
    let (word, phoneme) = split_once_colon(raw).ok_or_else(|| AppError::InvalidInput {
        msg: format!("--rule must be WORD:PHONEME (got '{raw}'). Example: --rule tomato:təˈmɑːtoʊ"),
        suggestion: None,
    })?;
    let word = word.trim();
    let phoneme = phoneme.trim();
    if word.is_empty() {
        return Err(AppError::InvalidInput {
            msg: format!("--rule missing WORD before ':' (got '{raw}')"),
            suggestion: None,
        });
    }
    if phoneme.is_empty() {
        return Err(AppError::InvalidInput {
            msg: format!("--rule missing PHONEME after ':' (got '{raw}')"),
            suggestion: None,
        });
    }
    Ok(serde_json::json!({
        "string_to_replace": word,
        "type": "phoneme",
        "phoneme": phoneme,
        "alphabet": "ipa",
    }))
}

/// Parse a `word:alias` CLI rule into the API body shape for an alias rule.
pub fn parse_alias_rule(raw: &str) -> Result<serde_json::Value, AppError> {
    let (word, alias) = split_once_colon(raw).ok_or_else(|| AppError::InvalidInput {
        msg: format!(
            "--alias-rule must be WORD:ALIAS (got '{raw}'). Example: --alias-rule SCUBA:scoo-buh"
        ),
        suggestion: None,
    })?;
    let word = word.trim();
    let alias = alias.trim();
    if word.is_empty() {
        return Err(AppError::InvalidInput {
            msg: format!("--alias-rule missing WORD before ':' (got '{raw}')"),
            suggestion: None,
        });
    }
    if alias.is_empty() {
        return Err(AppError::InvalidInput {
            msg: format!("--alias-rule missing ALIAS after ':' (got '{raw}')"),
            suggestion: None,
        });
    }
    Ok(serde_json::json!({
        "string_to_replace": word,
        "type": "alias",
        "alias": alias,
    }))
}

/// Collect `--rule` + `--alias-rule` invocations into a single Vec<Value>.
/// Errors out with exit 3 (invalid_input) if every bucket is empty, because
/// the API rejects an empty `rules` array anyway.
pub fn collect_rules(
    rules: Vec<String>,
    alias_rules: Vec<String>,
) -> Result<Vec<serde_json::Value>, AppError> {
    if rules.is_empty() && alias_rules.is_empty() {
        return Err(AppError::bad_input_with(
            "at least one --rule WORD:PHONEME or --alias-rule WORD:ALIAS is required",
            "elevenlabs dict add-rules NAME --rule word:phoneme  (repeat --rule or --alias-rule \
             as needed)",
        ));
    }
    let mut out = Vec::with_capacity(rules.len() + alias_rules.len());
    for r in &rules {
        out.push(parse_rule(r)?);
    }
    for r in &alias_rules {
        out.push(parse_alias_rule(r)?);
    }
    Ok(out)
}

/// `str::split_once(':')` but explicit — returned in a shape convenient for
/// the two helpers above.
fn split_once_colon(s: &str) -> Option<(&str, &str)> {
    s.split_once(':')
}
