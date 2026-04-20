//! `phone call` — place an outbound call via an agent. Dispatches to the
//! correct provider endpoint based on the phone number's provider field.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    from_id: String,
    to: String,
) -> Result<(), AppError> {
    // Determine provider type by looking up the phone.
    let list: serde_json::Value = client.get_json("/v1/convai/phone-numbers").await?;
    let arr = list
        .as_array()
        .cloned()
        .or_else(|| {
            list.get("phone_numbers")
                .and_then(|p| p.as_array())
                .cloned()
        })
        .unwrap_or_default();
    let phone = arr.iter().find(|p| {
        p.get("phone_number_id")
            .and_then(|v| v.as_str())
            .map(|s| s == from_id)
            .unwrap_or(false)
    });
    let provider = phone
        .and_then(|p| p.get("provider"))
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_lowercase();

    let path = match provider.as_str() {
        "twilio" => "/v1/convai/twilio/outbound-call",
        "sip_trunk" => "/v1/convai/sip-trunk/outbound-call",
        "" => {
            return Err(AppError::InvalidInput(format!(
                "phone number {from_id} not found in your account"
            )));
        }
        other => {
            return Err(AppError::InvalidInput(format!(
                "unsupported phone provider: {other}"
            )));
        }
    };

    let body = serde_json::json!({
        "agent_id": agent_id,
        "agent_phone_number_id": from_id,
        "to_number": to,
    });
    let resp: serde_json::Value = client.post_json(path, &body).await?;
    let result = serde_json::json!({
        "provider": provider,
        "agent_id": agent_id,
        "from_phone_number_id": from_id,
        "to": to,
        "response": resp,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} call placed via {} to {}",
            "+".green(),
            r["provider"].as_str().unwrap_or("").bold(),
            r["to"].as_str().unwrap_or("").bold()
        );
    });
    Ok(())
}
