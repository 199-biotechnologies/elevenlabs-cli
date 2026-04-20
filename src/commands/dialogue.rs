//! text-to-dialogue — multi-speaker generation with the `eleven_v3` flagship.
//!
//! Routes to one of four endpoints depending on flags:
//!   - default:                          POST /v1/text-to-dialogue
//!   - --stream:                         POST /v1/text-to-dialogue/stream
//!   - --with-timestamps:                POST /v1/text-to-dialogue/with-timestamps
//!   - --stream + --with-timestamps:     POST /v1/text-to-dialogue/stream/with-timestamps
//!
//! Request body shape (grounded against elevenlabs-python's
//! `BodyTextToDialogue*`):
//!
//!   {
//!     "inputs": [{ "text": "...", "voice_id": "..." }, ...],
//!     "model_id": "eleven_v3",
//!     "settings": { "stability": 0.5, "similarity_boost": 0.75, ... },
//!     "seed": <u32>,
//!     "apply_text_normalization": "auto"|"on"|"off",
//!     "language_code": "en"
//!   }
//!
//! Limits: up to 10 distinct voice IDs across all inputs, ~2000 total chars
//! (enforced client-side as a pre-flight; the server also enforces).
//!
//! Input parsing accepts two shapes on the CLI:
//!   1. `elevenlabs dialogue path/to/inputs.json` — JSON file
//!   2. `elevenlabs dialogue "Alice:voice_id_1:Hello" "Bob:voice_id_2:Hi"` —
//!      colon-delimited triples for small dialogues.
//!
//! The first positional is detected by extension / path existence — if the
//! first argument parses as a valid JSON file, we load it; otherwise every
//! positional is treated as a `label:voice_id:text` triple. The first
//! positional may also be `-` to read JSON from stdin.

use base64::Engine as _;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::cli::DialogueArgs;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

/// Upper-bound on unique voice IDs per request. Enforced by the API.
const MAX_UNIQUE_VOICES: usize = 10;
/// Advisory upper-bound on total characters across all inputs. Enforced by
/// the API (~2000 but not documented as a hard number).
const MAX_TOTAL_CHARS: usize = 2000;

#[derive(Serialize)]
struct DialogueResult {
    endpoint: String,
    model_id: String,
    inputs: usize,
    unique_voices: usize,
    characters: usize,
    output_format: String,
    output_path: Option<String>,
    alignment_path: Option<String>,
    bytes_written: usize,
}

pub async fn run(ctx: Ctx, args: DialogueArgs) -> Result<(), AppError> {
    // Parse the dialogue inputs before anything else so syntactic errors
    // never burn an API quota.
    let inputs = parse_inputs(&args).await?;

    if inputs.is_empty() {
        return Err(AppError::InvalidInput(
            "dialogue has no inputs — provide a JSON file or label:voice_id:text triples".into(),
        ));
    }

    let total_chars: usize = inputs.iter().map(|i| i.text.chars().count()).sum();
    let unique_voices: HashSet<&str> = inputs.iter().map(|i| i.voice_id.as_str()).collect();

    if unique_voices.len() > MAX_UNIQUE_VOICES {
        return Err(AppError::InvalidInput(format!(
            "dialogue has {} distinct voice IDs; the API accepts at most {MAX_UNIQUE_VOICES}. \
             Group lines by voice or drop speakers.",
            unique_voices.len()
        )));
    }
    if total_chars > MAX_TOTAL_CHARS {
        return Err(AppError::InvalidInput(format!(
            "dialogue is {total_chars} chars; the API accepts ~{MAX_TOTAL_CHARS}. \
             Split into multiple calls."
        )));
    }

    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    let model_id = args
        .model
        .clone()
        .unwrap_or_else(|| "eleven_v3".to_string());
    let output_format = args
        .format
        .clone()
        .unwrap_or_else(|| cfg.default_output_format());

    let body = build_body(&args, &inputs, &model_id);

    let mut query: Vec<(&str, String)> = vec![("output_format", output_format.clone())];
    if args.no_logging {
        query.push(("enable_logging", "false".to_string()));
    }
    if let Some(o) = args.optimize_streaming_latency {
        query.push(("optimize_streaming_latency", o.to_string()));
    }

    let (path, endpoint_label) = endpoint_for(args.stream, args.with_timestamps);

    // Branch: JSON-returning endpoints (both `with-timestamps` variants
    // return a JSON envelope with base64-encoded audio + alignment objects)
    // vs raw audio byte endpoints (the default + plain `stream`).
    if args.with_timestamps {
        let resp: serde_json::Value = client.post_json_with_query(path, &query, &body).await?;
        let audio = decode_timestamp_audio(&resp)?;
        let ext = crate::commands::tts::extension_for_format(&output_format);
        let out_path = crate::commands::resolve_output_path(args.output.clone(), "dialogue", ext);
        tokio::fs::write(&out_path, &audio)
            .await
            .map_err(AppError::Io)?;

        let alignment_path = args
            .save_timestamps
            .clone()
            .unwrap_or_else(|| format!("{}.timings.json", out_path.display()));
        // The shape differs slightly between `with-timestamps` (single
        // `alignment`) and the streaming variant (sequence of chunks). We
        // persist whatever we got verbatim minus the audio payloads.
        let alignment_obj = strip_audio_from_response(&resp);
        let pretty = serde_json::to_vec_pretty(&alignment_obj)
            .map_err(|e| AppError::Http(format!("serialize alignment: {e}")))?;
        tokio::fs::write(&alignment_path, pretty)
            .await
            .map_err(AppError::Io)?;

        let result = DialogueResult {
            endpoint: endpoint_label.to_string(),
            model_id: model_id.clone(),
            inputs: inputs.len(),
            unique_voices: unique_voices.len(),
            characters: total_chars,
            output_format,
            output_path: Some(out_path.display().to_string()),
            alignment_path: Some(alignment_path),
            bytes_written: audio.len(),
        };
        output::print_success_or(ctx, &result, print_human);
        return Ok(());
    }

    // Raw audio bytes path (default + plain --stream).
    let audio = client
        .post_json_bytes_with_query(path, &query, &body)
        .await?;
    let bytes_written = audio.len();

    if args.stdout {
        let mut out = tokio::io::stdout();
        out.write_all(&audio).await.map_err(AppError::Io)?;
        out.flush().await.map_err(AppError::Io)?;
        return Ok(());
    }

    let ext = crate::commands::tts::extension_for_format(&output_format);
    let out_path = crate::commands::resolve_output_path(args.output.clone(), "dialogue", ext);
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    let result = DialogueResult {
        endpoint: endpoint_label.to_string(),
        model_id,
        inputs: inputs.len(),
        unique_voices: unique_voices.len(),
        characters: total_chars,
        output_format,
        output_path: Some(out_path.display().to_string()),
        alignment_path: None,
        bytes_written,
    };
    output::print_success_or(ctx, &result, print_human);
    Ok(())
}

fn endpoint_for(stream: bool, with_timestamps: bool) -> (&'static str, &'static str) {
    match (stream, with_timestamps) {
        (true, true) => (
            "/v1/text-to-dialogue/stream/with-timestamps",
            "stream+with-timestamps",
        ),
        (true, false) => ("/v1/text-to-dialogue/stream", "stream"),
        (false, true) => ("/v1/text-to-dialogue/with-timestamps", "with-timestamps"),
        (false, false) => ("/v1/text-to-dialogue", "convert"),
    }
}

fn print_human(r: &DialogueResult) {
    use owo_colors::OwoColorize;
    let size_kb = r.bytes_written as f64 / 1024.0;
    println!(
        "{} {} ({:.1} KB, {} inputs, {} voices, {} chars, model={}, endpoint={})",
        "+".green(),
        r.output_path.as_deref().unwrap_or("(stdout)").bold(),
        size_kb,
        r.inputs,
        r.unique_voices,
        r.characters,
        r.model_id.dimmed(),
        r.endpoint.dimmed(),
    );
    if let Some(p) = &r.alignment_path {
        println!("  alignment: {}", p.bold());
    }
}

// ── Input parsing ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DialogueInput {
    pub text: String,
    pub voice_id: String,
    /// Speaker label (from the triple shape or JSON). Not sent to the API —
    /// kept for potential human-readable summaries.
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    pub label: Option<String>,
}

pub(crate) async fn parse_inputs(args: &DialogueArgs) -> Result<Vec<DialogueInput>, AppError> {
    // Preferred path: `--input <file>` or `--input -` for stdin JSON.
    if let Some(source) = &args.input {
        let raw = if source == "-" {
            let mut s = String::new();
            tokio::io::stdin()
                .read_to_string(&mut s)
                .await
                .map_err(AppError::Io)?;
            s
        } else {
            tokio::fs::read_to_string(Path::new(source))
                .await
                .map_err(|e| AppError::InvalidInput(format!("read {source}: {e}")))?
        };
        return parse_json_inputs(&raw);
    }

    if args.positional.is_empty() {
        return Err(AppError::InvalidInput(
            "dialogue requires either --input <json_file>, `--input -` for stdin, \
             or one or more `label:voice_id:text` positional triples"
                .into(),
        ));
    }

    // Shape 1: single positional that looks like a JSON file path.
    if args.positional.len() == 1 {
        let first = &args.positional[0];
        if looks_like_json_file(first) {
            let raw = tokio::fs::read_to_string(Path::new(first))
                .await
                .map_err(|e| AppError::InvalidInput(format!("read {first}: {e}")))?;
            return parse_json_inputs(&raw);
        }
        if first == "-" {
            let mut s = String::new();
            tokio::io::stdin()
                .read_to_string(&mut s)
                .await
                .map_err(AppError::Io)?;
            return parse_json_inputs(&s);
        }
    }

    // Shape 2: colon-delimited triples.
    parse_triples(&args.positional)
}

fn looks_like_json_file(p: &str) -> bool {
    // A path is "JSON-looking" if the extension is .json AND the file
    // exists. We avoid stat-ing every positional because triples like
    // "Alice:voice_id:hi" can sometimes have colons and we must not shadow
    // them with a missing-file error.
    let path = Path::new(p);
    let is_json_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("json"))
        .unwrap_or(false);
    is_json_ext && path.exists()
}

fn parse_json_inputs(raw: &str) -> Result<Vec<DialogueInput>, AppError> {
    let v: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| AppError::InvalidInput(format!("dialogue input is not valid JSON: {e}")))?;
    // Accept two shapes for convenience:
    //   - top-level array: [ { text, voice_id }, ... ]
    //   - top-level object with "inputs": { "inputs": [ ... ] }
    let arr = if let Some(arr) = v.as_array() {
        arr.clone()
    } else if let Some(arr) = v.get("inputs").and_then(|x| x.as_array()) {
        arr.clone()
    } else {
        return Err(AppError::InvalidInput(
            "dialogue JSON must be either an array of {text, voice_id} objects \
             or an object with an `inputs` array"
                .into(),
        ));
    };

    let mut out: Vec<DialogueInput> = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        let text = item
            .get("text")
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                AppError::InvalidInput(format!("dialogue input[{i}] missing string `text`"))
            })?
            .to_string();
        let voice_id = item
            .get("voice_id")
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                AppError::InvalidInput(format!("dialogue input[{i}] missing string `voice_id`"))
            })?
            .to_string();
        if text.trim().is_empty() {
            return Err(AppError::InvalidInput(format!(
                "dialogue input[{i}] has empty `text`"
            )));
        }
        if voice_id.trim().is_empty() {
            return Err(AppError::InvalidInput(format!(
                "dialogue input[{i}] has empty `voice_id`"
            )));
        }
        let label = item
            .get("label")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());
        out.push(DialogueInput {
            text,
            voice_id,
            label,
        });
    }
    Ok(out)
}

fn parse_triples(positionals: &[String]) -> Result<Vec<DialogueInput>, AppError> {
    let mut out: Vec<DialogueInput> = Vec::with_capacity(positionals.len());
    for (i, p) in positionals.iter().enumerate() {
        // `label:voice_id:text` — splitn 3 so `text` may contain colons.
        let mut parts = p.splitn(3, ':');
        let label = parts.next().unwrap_or("").trim().to_string();
        let voice_id = parts
            .next()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let text = parts.next().unwrap_or("").to_string();
        if label.is_empty() || voice_id.is_empty() || text.trim().is_empty() {
            return Err(AppError::InvalidInput(format!(
                "dialogue positional[{i}] '{p}' must be `label:voice_id:text`. \
                 If this is a JSON file, ensure the path ends in `.json` and exists, \
                 or pass it via --input."
            )));
        }
        out.push(DialogueInput {
            text,
            voice_id,
            label: Some(label),
        });
    }
    Ok(out)
}

// ── Body / response helpers ────────────────────────────────────────────────

fn build_body(args: &DialogueArgs, inputs: &[DialogueInput], model_id: &str) -> serde_json::Value {
    let inputs_json: Vec<serde_json::Value> = inputs
        .iter()
        .map(|i| {
            serde_json::json!({
                "text": i.text,
                "voice_id": i.voice_id,
            })
        })
        .collect();

    let mut settings = serde_json::Map::new();
    if let Some(v) = args.stability {
        settings.insert("stability".into(), serde_json::json!(v));
    }
    if let Some(v) = args.similarity {
        settings.insert("similarity_boost".into(), serde_json::json!(v));
    }
    if let Some(v) = args.style {
        settings.insert("style".into(), serde_json::json!(v));
    }
    if let Some(v) = args.speaker_boost {
        settings.insert("use_speaker_boost".into(), serde_json::json!(v));
    }

    let mut body = serde_json::Map::new();
    body.insert("inputs".into(), serde_json::Value::Array(inputs_json));
    body.insert(
        "model_id".into(),
        serde_json::Value::String(model_id.to_string()),
    );
    if !settings.is_empty() {
        body.insert("settings".into(), serde_json::Value::Object(settings));
    }
    if let Some(seed) = args.seed {
        body.insert("seed".into(), serde_json::json!(seed));
    }
    if let Some(lang) = &args.language {
        body.insert(
            "language_code".into(),
            serde_json::Value::String(lang.clone()),
        );
    }
    if let Some(norm) = &args.apply_text_normalization {
        body.insert(
            "apply_text_normalization".into(),
            serde_json::Value::String(norm.clone()),
        );
    }
    serde_json::Value::Object(body)
}

fn decode_timestamp_audio(resp: &serde_json::Value) -> Result<Vec<u8>, AppError> {
    // Two accepted shapes per observed server responses:
    //   1. `with-timestamps`:             { audio_base64, alignment, normalized_alignment }
    //   2. `stream/with-timestamps`:      { chunks: [{ audio_base64, alignment, ... }, ...] }
    //      (we concatenate the audio from each chunk)
    if let Some(audio_b64) = resp.get("audio_base64").and_then(|v| v.as_str()) {
        return base64::engine::general_purpose::STANDARD
            .decode(audio_b64)
            .map_err(|e| AppError::Http(format!("decode audio base64: {e}")));
    }
    if let Some(chunks) = resp.get("chunks").and_then(|v| v.as_array()) {
        let mut all = Vec::new();
        for (i, c) in chunks.iter().enumerate() {
            let b64 = c
                .get("audio_base64")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AppError::Http(format!("dialogue response chunk[{i}] missing audio_base64"))
                })?;
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| AppError::Http(format!("decode chunk[{i}] audio base64: {e}")))?;
            all.extend_from_slice(&decoded);
        }
        return Ok(all);
    }
    Err(AppError::Http(
        "dialogue with-timestamps response had no audio payload (expected `audio_base64` \
         or `chunks[]`)"
            .into(),
    ))
}

fn strip_audio_from_response(resp: &serde_json::Value) -> serde_json::Value {
    // Best-effort: clone and wipe any `audio_base64` fields so the alignment
    // JSON we write to disk stays diffable.
    let mut clone = resp.clone();
    if let Some(obj) = clone.as_object_mut() {
        obj.remove("audio_base64");
        if let Some(chunks) = obj.get_mut("chunks").and_then(|v| v.as_array_mut()) {
            for c in chunks {
                if let Some(cm) = c.as_object_mut() {
                    cm.remove("audio_base64");
                }
            }
        }
    }
    clone
}
