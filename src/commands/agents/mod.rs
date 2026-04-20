//! Conversational AI agents — list, show, create, update, duplicate, delete,
//! knowledge base attachment, and tool management.

pub mod create;
pub mod delete;
pub mod duplicate;
pub mod knowledge;
pub mod list;
pub mod show;
pub mod tools;
pub mod update;

use crate::cli::AgentsAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::Ctx;

pub async fn dispatch(ctx: Ctx, action: AgentsAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    match action {
        AgentsAction::List => list::run(ctx, &client).await,
        AgentsAction::Show { agent_id } => show::run(ctx, &client, &agent_id).await,
        AgentsAction::Create {
            name,
            system_prompt,
            first_message,
            voice_id,
            language,
            llm,
            temperature,
            model_id,
        } => {
            create::run(
                ctx,
                &cfg,
                &client,
                name,
                system_prompt,
                first_message,
                voice_id,
                language,
                llm,
                temperature,
                model_id,
            )
            .await
        }
        AgentsAction::Update { agent_id, patch } => {
            update::run(ctx, &client, agent_id, patch).await
        }
        AgentsAction::Duplicate { agent_id, name } => {
            duplicate::run(ctx, &client, agent_id, name).await
        }
        AgentsAction::Delete { agent_id } => delete::run(ctx, &client, &agent_id).await,
        AgentsAction::AddKnowledge {
            agent_id,
            name,
            url,
            file,
            text,
        } => knowledge::add(ctx, &client, agent_id, name, url, file, text).await,
        AgentsAction::Tools { action } => tools::dispatch(ctx, &client, action).await,
    }
}
