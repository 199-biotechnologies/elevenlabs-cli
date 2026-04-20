//! music stem-separation — POST /v1/music/stem-separation
//!
//! Splits a track into stems (vocals / drums / bass / other by default).
//! Accepts either:
//!   - a local audio file (multipart upload), or
//!   - a `song_id` from `music upload` (JSON form field).
//!
//! The response is a JSON object keyed by stem name, each holding
//! base64-encoded audio. We decode and write one file per stem into the
//! chosen output directory.

use std::path::{Path, PathBuf};

use base64::Engine as _;

use crate::cli::StemSeparationArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    args: StemSeparationArgs,
) -> Result<(), AppError> {
    // Differentiate `<song_id>` from `<file>`: anything with a path
    // separator or pointing to an existing file is treated as a file;
    // everything else is assumed to be a song_id.
    let source = args.source.trim();
    if source.is_empty() {
        return Err(AppError::InvalidInput(
            "provide a local audio file or a song_id".into(),
        ));
    }
    let as_path = Path::new(source);
    let is_file =
        as_path.is_file() || source.contains('/') || source.contains('\\') || source.contains('.');

    let mut form = reqwest::multipart::Form::new();
    if is_file {
        if !as_path.exists() {
            return Err(AppError::InvalidInput(format!(
                "file does not exist: {}",
                as_path.display()
            )));
        }
        let bytes = crate::commands::read_file_bytes(as_path).await?;
        let mime = crate::commands::mime_for_path(as_path);
        let filename = as_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "audio".to_string());
        let file_part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str(&mime)
            .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;
        form = form.part("file", file_part);
    } else {
        form = form.text("song_id", source.to_string());
    }
    for stem in &args.stems {
        form = form.text("stems", stem.clone());
    }

    let resp: serde_json::Value = client
        .post_multipart_json("/v1/music/stem-separation", form)
        .await?;

    // Response shape: { "stems": { "vocals": "<base64>", ... }, ... } or
    // a flat map. Normalise both.
    let stems_obj = resp
        .get("stems")
        .and_then(|v| v.as_object())
        .cloned()
        .or_else(|| resp.as_object().cloned())
        .ok_or_else(|| {
            AppError::Http("stem-separation response is not an object of base64 stems".into())
        })?;

    let out_dir = args
        .output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("stems_{}", crate::commands::now_timestamp())));
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(AppError::Io)?;

    let mut written: Vec<serde_json::Value> = Vec::new();
    for (name, value) in stems_obj {
        let Some(b64) = value.as_str() else {
            continue;
        };
        let audio = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| AppError::Http(format!("decode stem '{name}' base64: {e}")))?;
        let file = out_dir.join(format!("{name}.mp3"));
        tokio::fs::write(&file, &audio)
            .await
            .map_err(AppError::Io)?;
        written.push(serde_json::json!({
            "stem": name,
            "path": file.display().to_string(),
            "bytes": audio.len(),
        }));
    }

    let result = serde_json::json!({
        "source": source,
        "output_dir": out_dir.display().to_string(),
        "stems_requested": args.stems,
        "stems_written": written,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} stems -> {}",
            "+".green(),
            r["stems_written"].as_array().map(|a| a.len()).unwrap_or(0),
            r["output_dir"].as_str().unwrap_or("").bold()
        );
    });
    Ok(())
}
