//! music video-to-music — POST /v1/music/video-to-music
//!
//! NEW Apr 1, 2026. Takes a video file as input and generates a musical
//! score that matches the visual content. Multipart: `file` = video.
//! Optional text hints: `--description`, `--tags`. Returns audio bytes
//! (same shape as compose) with output_format controllable via query.

use std::path::Path;

use crate::cli::VideoToMusicArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    cfg: &crate::config::AppConfig,
    client: &ElevenLabsClient,
    args: VideoToMusicArgs,
) -> Result<(), AppError> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(AppError::InvalidInput(format!(
            "video file does not exist: {}",
            path.display()
        )));
    }

    let bytes = crate::commands::read_file_bytes(path).await?;
    let mime = crate::commands::mime_for_path(path);
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "video".to_string());
    let file_part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(&mime)
        .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;

    let mut form = reqwest::multipart::Form::new().part("file", file_part);
    if let Some(desc) = &args.description {
        form = form.text("description", desc.clone());
    }
    for tag in &args.tags {
        form = form.text("tags", tag.clone());
    }
    if let Some(m) = &args.model {
        form = form.text("model_id", m.clone());
    }

    let output_format = args.format.unwrap_or_else(|| cfg.default_output_format());
    let query = [("output_format", output_format.as_str())];

    let audio = client
        .post_multipart_bytes_with_query("/v1/music/video-to-music", &query, form)
        .await?;
    let bytes_written = audio.len();

    let ext = crate::commands::tts::extension_for_format(&output_format);
    let out_path = crate::commands::resolve_output_path(args.output, "music", ext);
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "input": path.display().to_string(),
        "output": out_path.display().to_string(),
        "output_format": output_format,
        "description": args.description,
        "tags": args.tags,
        "bytes_written": bytes_written,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} ({:.1} KB)",
            "+".green(),
            r["output"].as_str().unwrap_or("").bold(),
            r["bytes_written"].as_f64().unwrap_or(0.0) / 1024.0
        );
    });
    Ok(())
}
