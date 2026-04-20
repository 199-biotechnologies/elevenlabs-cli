//! `phone whatsapp message` — POST /v1/convai/whatsapp/outbound-message
//!
//! Supply either `--text <str>` (free-form) or `--template <name>` (a
//! pre-approved WhatsApp template). Exactly one is required.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    whatsapp_account: String,
    recipient: String,
    text: Option<String>,
    template: Option<String>,
) -> Result<(), AppError> {
    match (&text, &template) {
        (Some(_), Some(_)) => {
            return Err(AppError::InvalidInput(
                "pass only one of --text or --template, not both".into(),
            ));
        }
        (None, None) => {
            return Err(AppError::InvalidInput(
                "either --text <str> or --template <name> is required".into(),
            ));
        }
        _ => {}
    }

    let mut body = serde_json::Map::new();
    body.insert("agent_id".into(), serde_json::Value::String(agent_id));
    body.insert(
        "whatsapp_account_id".into(),
        serde_json::Value::String(whatsapp_account),
    );
    body.insert(
        "recipient_phone_number".into(),
        serde_json::Value::String(recipient.clone()),
    );
    if let Some(t) = text {
        body.insert("text".into(), serde_json::Value::String(t));
    }
    if let Some(t) = template {
        body.insert("template_name".into(), serde_json::Value::String(t));
    }

    let resp: serde_json::Value = client
        .post_json(
            "/v1/convai/whatsapp/outbound-message",
            &serde_json::Value::Object(body),
        )
        .await?;
    let result = serde_json::json!({
        "recipient": recipient,
        "response": resp,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} WhatsApp message sent to {}",
            "+".green(),
            r["recipient"].as_str().unwrap_or("").bold()
        );
    });
    Ok(())
}
