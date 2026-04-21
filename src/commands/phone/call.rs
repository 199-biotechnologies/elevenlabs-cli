//! `phone call` — place an outbound call via an agent. Dispatches to the
//! correct provider endpoint based on the phone number's provider field.

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    from_id: String,
    to: String,
    dynamic_variables: Option<String>,
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
            return Err(AppError::InvalidInput {
                msg: format!("phone number {from_id} not found in your account"),
                suggestion: None,
            });
        }
        other => {
            return Err(AppError::InvalidInput {
                msg: format!("unsupported phone provider: {other}"),
                suggestion: None,
            });
        }
    };

    let mut body = serde_json::json!({
        "agent_id": agent_id,
        "agent_phone_number_id": from_id,
        "to_number": to,
    });

    if let Some(raw) = dynamic_variables.as_deref() {
        let vars = parse_dynamic_variables(raw).await?;
        body["conversation_initiation_client_data"] =
            serde_json::json!({ "dynamic_variables": vars });
    }

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

/// Parse `--dynamic-variables` value. Accepts either a raw JSON object or
/// `@path/to/file.json` (leading `@` loads the file). The resulting value
/// must be a JSON object; arrays/strings/primitives error out because the
/// agent template engine only interpolates `{{key}}` lookups.
async fn parse_dynamic_variables(raw: &str) -> Result<serde_json::Value, AppError> {
    let text = if let Some(rest) = raw.strip_prefix('@') {
        let path = Path::new(rest);
        if !path.exists() {
            return Err(AppError::bad_input_with(
                format!("dynamic-variables file does not exist: {}", path.display()),
                format!(
                    "Either drop the leading `@` to pass a literal JSON string, or point at an \
                     existing file: --dynamic-variables @{}",
                    path.display()
                ),
            ));
        }
        tokio::fs::read_to_string(path)
            .await
            .map_err(AppError::Io)?
    } else {
        raw.to_string()
    };

    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        AppError::bad_input_with(
            format!("--dynamic-variables is not valid JSON: {e}"),
            "Pass a JSON object like --dynamic-variables '{\"name\":\"Alex\"}'. Use @file.json \
             to load from disk if the JSON is large or contains special shell characters."
                .to_string(),
        )
    })?;
    if !val.is_object() {
        return Err(AppError::bad_input_with(
            format!(
                "--dynamic-variables must be a JSON object (got {})",
                val_kind(&val)
            ),
            "Dynamic variables interpolate {{key}} placeholders in the agent prompt, so the \
             top level must be an object: --dynamic-variables '{\"name\":\"Alex\",\"plan\":\"pro\"}'"
                .to_string(),
        ));
    }
    Ok(val)
}

fn val_kind(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}
