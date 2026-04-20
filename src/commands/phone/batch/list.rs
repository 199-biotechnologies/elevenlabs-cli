//! `phone batch list` — GET /v1/convai/batch-calling/workspace

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    page_size: Option<u32>,
    cursor: Option<String>,
    status: Option<String>,
    agent_id: Option<String>,
) -> Result<(), AppError> {
    let mut params: Vec<(&str, String)> = Vec::new();
    if let Some(ps) = page_size {
        params.push(("page_size", ps.min(100).to_string()));
    }
    if let Some(c) = cursor {
        params.push(("cursor", c));
    }
    if let Some(s) = status {
        params.push(("status", s));
    }
    if let Some(a) = agent_id {
        params.push(("agent_id", a));
    }

    let resp: serde_json::Value = client
        .get_json_with_query("/v1/convai/batch-calling/workspace", &params)
        .await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let items = v
            .get("batch_calls")
            .or_else(|| v.get("batches"))
            .and_then(|d| d.as_array())
            .cloned()
            .or_else(|| v.as_array().cloned())
            .unwrap_or_default();
        if items.is_empty() {
            println!("(no batches)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec![
            "Batch ID",
            "Name",
            "Status",
            "Agent",
            "Total",
            "Completed",
            "Created",
        ]);
        for b in &items {
            t.add_row(vec![
                b.get("id")
                    .or_else(|| b.get("batch_id"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                b.get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                b.get("status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                b.get("agent_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                b.get("total_calls_dispatched")
                    .or_else(|| b.get("total"))
                    .and_then(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                b.get("total_calls_completed")
                    .or_else(|| b.get("completed"))
                    .and_then(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                b.get("created_at_unix")
                    .and_then(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
            ]);
        }
        println!("{t}");
        if let Some(next) = v.get("next_cursor").and_then(|x| x.as_str()) {
            if !next.is_empty() {
                println!("\nnext page: --cursor {next}");
            }
        }
    });
    Ok(())
}
