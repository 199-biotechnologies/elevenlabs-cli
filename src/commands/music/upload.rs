//! music upload — POST /v1/music/upload
//!
//! Multipart upload of an existing audio file so it can be referenced by
//! `song_id` in later inpainting / regeneration calls. Returns the
//! server-assigned `song_id`. Optional `--composition-plan <file>` lets
//! the user attach a plan alongside the audio.

use std::path::Path;

use crate::cli::UploadArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, args: UploadArgs) -> Result<(), AppError> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(AppError::InvalidInput(format!(
            "file does not exist: {}",
            path.display()
        )));
    }

    let bytes = crate::commands::read_file_bytes(path).await?;
    let mime = crate::commands::mime_for_path(path);
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "audio".to_string());
    let file_part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(&mime)
        .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;

    let mut form = reqwest::multipart::Form::new().part("file", file_part);
    if let Some(name) = &args.name {
        form = form.text("name", name.clone());
    }
    if let Some(plan_path) = &args.composition_plan {
        let content = tokio::fs::read_to_string(plan_path)
            .await
            .map_err(AppError::Io)?;
        // Validate it parses before shipping so we fail early with a good error.
        let _: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            AppError::InvalidInput(format!("--composition-plan is not valid JSON: {e}"))
        })?;
        form = form.text("composition_plan", content);
    }

    let resp: serde_json::Value = client.post_multipart_json("/v1/music/upload", form).await?;

    // Response shape: { song_id, ... }
    let song_id = resp
        .get("song_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let result = serde_json::json!({
        "input": path.display().to_string(),
        "song_id": song_id,
        "name": args.name,
        "response": resp,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} uploaded {} song_id={}",
            "+".green(),
            r["input"].as_str().unwrap_or("").bold(),
            r["song_id"].as_str().unwrap_or("(unknown)").dimmed()
        );
    });
    Ok(())
}
