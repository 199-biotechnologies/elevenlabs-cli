//! `agents tools` — manage workspace-level tools that agents can call
//! (system, client, webhook, mcp). The API surface is too wide to model
//! each field as a flag; create/update accept a JSON file path and PATCH
//! the body through verbatim.

pub mod create;
pub mod delete;
pub mod deps;
pub mod list;
pub mod show;
pub mod update;

use crate::cli::AgentsToolsAction;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::Ctx;

pub async fn dispatch(
    ctx: Ctx,
    client: &ElevenLabsClient,
    action: AgentsToolsAction,
) -> Result<(), AppError> {
    match action {
        AgentsToolsAction::List => list::run(ctx, client).await,
        AgentsToolsAction::Show { tool_id } => show::run(ctx, client, &tool_id).await,
        AgentsToolsAction::Create { config } => create::run(ctx, client, config).await,
        AgentsToolsAction::Update { tool_id, patch } => {
            update::run(ctx, client, tool_id, patch).await
        }
        AgentsToolsAction::Delete { tool_id, yes } => delete::run(ctx, client, tool_id, yes).await,
        AgentsToolsAction::Deps { tool_id } => deps::run(ctx, client, &tool_id).await,
    }
}
