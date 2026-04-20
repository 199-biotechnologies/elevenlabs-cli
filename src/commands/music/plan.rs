//! music plan — POST /v1/music/plan
//!
//! Returns a JSON composition plan (sections + metadata). Free endpoint,
//! subject to rate limits. The plan can be piped back into `music compose
//! --composition-plan <file>`.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    prompt: String,
    length_ms: Option<u32>,
    model: Option<String>,
) -> Result<(), AppError> {
    if prompt.trim().is_empty() {
        return Err(AppError::InvalidInput {
            msg: "prompt is empty".into(),
            suggestion: None,
        });
    }
    let mut body = serde_json::Map::new();
    body.insert("prompt".into(), serde_json::Value::String(prompt));
    if let Some(ms) = length_ms {
        body.insert("music_length_ms".into(), serde_json::json!(ms));
    }
    if let Some(m) = model {
        body.insert("model_id".into(), serde_json::Value::String(m));
    }
    let resp: serde_json::Value = client
        .post_json("/v1/music/plan", &serde_json::Value::Object(body))
        .await?;
    output::print_success_or(ctx, &resp, |v| {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    });
    Ok(())
}
