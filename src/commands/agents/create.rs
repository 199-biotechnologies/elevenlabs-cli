//! `agents create` — build a new conversational AI agent from a minimal
//! set of flags. The full `conversation_config` + `platform_settings`
//! scaffolding is filled in with sensible defaults so agents don't have to
//! know the entire schema to spin up a working agent.

use crate::client::ElevenLabsClient;
use crate::config::AppConfig;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    ctx: Ctx,
    cfg: &AppConfig,
    client: &ElevenLabsClient,
    name: String,
    system_prompt: String,
    first_message: Option<String>,
    voice_id: Option<String>,
    language: String,
    llm: String,
    temperature: f32,
    model_id: String,
) -> Result<(), AppError> {
    let voice_id = voice_id.unwrap_or_else(|| cfg.default_voice_id());

    let conversation_config = serde_json::json!({
        "agent": {
            "language": language,
            "prompt": {
                "prompt": system_prompt,
                "llm": llm,
                "tools": [{"type": "system", "name": "end_call", "description": ""}],
                "knowledge_base": [],
                "temperature": temperature,
            },
            "first_message": first_message,
            "dynamic_variables": { "dynamic_variable_placeholders": {} }
        },
        "asr": {
            "quality": "high",
            "provider": "elevenlabs",
            "user_input_audio_format": "pcm_16000",
            "keywords": []
        },
        "tts": {
            "voice_id": voice_id,
            "model_id": model_id,
            "agent_output_audio_format": "pcm_16000",
            "optimize_streaming_latency": 3,
            "stability": 0.5,
            "similarity_boost": 0.8
        },
        "turn": { "turn_timeout": 7 },
        "conversation": {
            "max_duration_seconds": 300,
            "client_events": [
                "audio", "interruption", "user_transcript",
                "agent_response", "agent_response_correction"
            ]
        },
        "language_presets": {},
        "is_blocked_ivc": false,
        "is_blocked_non_ivc": false
    });

    let platform_settings = serde_json::json!({
        "widget": {
            "variant": "full",
            "avatar": { "type": "orb", "color_1": "#6DB035", "color_2": "#F5CABB" },
            "feedback_mode": "during"
        },
        "evaluation": {},
        "auth": { "allowlist": [] },
        "overrides": {},
        "call_limits": { "agent_concurrency_limit": -1, "daily_limit": 100000 },
        "privacy": {
            "record_voice": true,
            "retention_days": 730,
            "delete_transcript_and_pii": true,
            "delete_audio": true,
            "apply_to_existing_conversations": false
        },
        "data_collection": {}
    });

    let body = serde_json::json!({
        "name": name,
        "conversation_config": conversation_config,
        "platform_settings": platform_settings,
    });

    let resp: serde_json::Value = client.post_json("/v1/convai/agents/create", &body).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} created agent {} ({})",
            "+".green(),
            name.bold(),
            v.get("agent_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .dimmed()
        );
    });
    Ok(())
}
