//! music detailed — POST /v1/music/detailed
//!
//! Returns rich JSON with the generated audio (base64) plus metadata:
//! bpm, time_signature, sections (intro / verse / chorus boundaries),
//! key, genre, mood. Useful for downstream editing or for agents that
//! want to reason about the composition.
//!
//! Splits the response into two files: the audio track (default extension
//! inferred from the output_format query param) and a companion JSON file
//! holding everything else.

use base64::Engine as _;

use crate::cli::DetailedArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    cfg: &crate::config::AppConfig,
    client: &ElevenLabsClient,
    args: DetailedArgs,
) -> Result<(), AppError> {
    let body = super::build_compose_body(
        args.prompt.as_deref(),
        args.length_ms,
        args.composition_plan.as_deref(),
        args.model.as_deref(),
        args.force_instrumental,
        args.seed,
        args.respect_sections_durations,
        args.store_for_inpainting,
        args.sign_with_c2pa,
    )
    .await?;

    let output_format = args.format.unwrap_or_else(|| cfg.default_output_format());
    let query = [("output_format", output_format.as_str())];

    let resp: serde_json::Value = client
        .post_json_with_query("/v1/music/detailed", &query, &body)
        .await?;

    // Extract the audio. The API returns it as `audio_base64`; some
    // responses may use `audio` — support both so we don't crash if the
    // field gets renamed.
    let audio_b64 = resp
        .get("audio_base64")
        .or_else(|| resp.get("audio"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::Http("music/detailed response missing audio_base64 field".into())
        })?;
    let audio = base64::engine::general_purpose::STANDARD
        .decode(audio_b64)
        .map_err(|e| AppError::Http(format!("decode audio base64: {e}")))?;

    let ext = crate::commands::tts::extension_for_format(&output_format);
    let out_path = crate::commands::resolve_output_path(args.output, "music", ext);
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    // Split out metadata: everything except the raw audio bytes. Default
    // path is <audio>.metadata.json so a single `-o track.mp3` argument
    // produces both files side by side.
    let metadata_path = args
        .save_metadata
        .unwrap_or_else(|| format!("{}.metadata.json", out_path.display()));
    let mut metadata = resp.clone();
    if let Some(obj) = metadata.as_object_mut() {
        obj.remove("audio_base64");
        obj.remove("audio");
    }
    let pretty = serde_json::to_vec_pretty(&metadata)
        .map_err(|e| AppError::Http(format!("serialize metadata: {e}")))?;
    tokio::fs::write(&metadata_path, pretty)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "prompt": args.prompt,
        "composition_plan_file": args.composition_plan,
        "length_ms": args.length_ms,
        "seed": args.seed,
        "force_instrumental": args.force_instrumental,
        "output": out_path.display().to_string(),
        "metadata_path": metadata_path,
        "output_format": output_format,
        "bytes_written": audio.len(),
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} ({:.1} KB)",
            "+".green(),
            r["output"].as_str().unwrap_or("").bold(),
            r["bytes_written"].as_f64().unwrap_or(0.0) / 1024.0,
        );
        println!("  metadata: {}", r["metadata_path"].as_str().unwrap_or(""));
    });
    Ok(())
}
