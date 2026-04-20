//! `phone whatsapp call` — POST /v1/convai/whatsapp/outbound-call

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    whatsapp_account: String,
    recipient: String,
) -> Result<(), AppError> {
    let body = serde_json::json!({
        "agent_id": agent_id,
        "whatsapp_account_id": whatsapp_account,
        "recipient_phone_number": recipient,
    });
    let resp: serde_json::Value = client
        .post_json("/v1/convai/whatsapp/outbound-call", &body)
        .await?;
    let result = serde_json::json!({
        "agent_id": agent_id,
        "whatsapp_account_id": whatsapp_account,
        "recipient": recipient,
        "response": resp,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} WhatsApp call placed to {}",
            "+".green(),
            r["recipient"].as_str().unwrap_or("").bold()
        );
    });
    Ok(())
}
