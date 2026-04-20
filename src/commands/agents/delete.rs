//! `agents delete` — remove an agent. Irreversible server-side.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, agent_id: &str) -> Result<(), AppError> {
    let path = format!("/v1/convai/agents/{agent_id}");
    client.delete(&path).await?;
    let result = serde_json::json!({ "agent_id": agent_id, "deleted": true });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} deleted agent {}", "-".red(), agent_id.dimmed());
    });
    Ok(())
}
