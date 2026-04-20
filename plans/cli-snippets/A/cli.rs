// plans/cli-snippets/A/cli.rs
// ─────────────────────────────────────────────────────────────────────────────
// REPLACE the existing `AgentsAction` enum in src/cli.rs wholesale with the
// version below. Changes vs the old enum:
//   1. `Create` defaults bumped (`--llm` → gemini-3.1-flash-lite-preview,
//      `--model-id` → eleven_flash_v2_5). Doc comments updated to match.
//   2. New `Update` variant (PATCH partial config via JSON file).
//   3. New `Duplicate` variant (POST /duplicate).
//   4. New `Tools { action }` variant gating the AgentsToolsAction enum.
//
// Also APPEND the new `AgentsToolsAction` enum at the end of the file (or
// alongside the other `#[derive(Subcommand, Debug, Clone)]` blocks).
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum AgentsAction {
    /// List agents
    #[command(visible_alias = "ls")]
    List,

    /// Get agent details
    #[command(visible_alias = "get")]
    Show {
        /// Agent ID
        agent_id: String,
    },

    /// Create a new agent
    #[command(visible_alias = "new")]
    Create {
        /// Agent name
        name: String,

        /// System prompt
        #[arg(long)]
        system_prompt: String,

        /// First message the agent says
        #[arg(long)]
        first_message: Option<String>,

        /// Voice ID for the agent
        #[arg(long)]
        voice_id: Option<String>,

        /// Language ISO 639-1 code (default en)
        #[arg(long, default_value = "en")]
        language: String,

        /// LLM to use (default gemini-3.1-flash-lite-preview)
        #[arg(long, default_value = "gemini-3.1-flash-lite-preview")]
        llm: String,

        /// Temperature 0.0-1.0
        #[arg(long, default_value = "0.5")]
        temperature: f32,

        /// TTS model ID (default eleven_flash_v2_5)
        #[arg(long, default_value = "eleven_flash_v2_5")]
        model_id: String,
    },

    /// Update (PATCH) an agent's config from a JSON file
    Update {
        /// Agent ID
        agent_id: String,
        /// Path to a JSON file whose contents are the PATCH body.
        /// Pass-through — lets you edit system_prompt, voice_id, tools,
        /// knowledge_base, etc. without recreating the agent.
        #[arg(long, value_name = "PATH")]
        patch: String,
    },

    /// Duplicate an agent (clone config to a new agent_id)
    Duplicate {
        /// Agent ID to duplicate
        agent_id: String,
        /// Optional name override for the new agent
        #[arg(long)]
        name: Option<String>,
    },

    /// Delete an agent
    #[command(visible_alias = "rm")]
    Delete {
        /// Agent ID
        agent_id: String,
    },

    /// Add a knowledge base document and attach it to an agent
    AddKnowledge {
        /// Agent ID
        agent_id: String,

        /// Document name
        name: String,

        /// Source URL (one of: --url, --file, --text)
        #[arg(long)]
        url: Option<String>,

        /// Source file path
        #[arg(long)]
        file: Option<String>,

        /// Source text
        #[arg(long)]
        text: Option<String>,
    },

    /// Manage workspace-level tools
    Tools {
        #[command(subcommand)]
        action: AgentsToolsAction,
    },
}

// ── Agents tools ──────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum AgentsToolsAction {
    /// List tools
    #[command(visible_alias = "ls")]
    List,

    /// Show full tool config
    #[command(visible_alias = "get")]
    Show {
        /// Tool ID
        tool_id: String,
    },

    /// Create a tool from a JSON config file
    #[command(visible_alias = "new")]
    Create {
        /// Path to a JSON file that becomes the POST body verbatim.
        /// The tools API surface is wide (system/client/webhook/mcp tool
        /// types, many fields each) — pass the JSON directly instead of
        /// modelling every field as a flag.
        #[arg(long, value_name = "PATH")]
        config: String,
    },

    /// Update (PATCH) a tool from a JSON file
    Update {
        /// Tool ID
        tool_id: String,
        /// Path to a JSON file whose contents are the PATCH body.
        #[arg(long, value_name = "PATH")]
        patch: String,
    },

    /// Delete a tool (requires --yes)
    #[command(visible_alias = "rm")]
    Delete {
        /// Tool ID
        tool_id: String,
        /// Confirm deletion. Without --yes the command errors out.
        #[arg(long)]
        yes: bool,
    },

    /// List agents that depend on this tool
    #[command(visible_alias = "dependents")]
    Deps {
        /// Tool ID
        tool_id: String,
    },
}
