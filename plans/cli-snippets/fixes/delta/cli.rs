// ─── cli.rs snippet: AgentsAction::Delete needs a `yes` flag ───────────────
//
// CURRENT (src/cli.rs, inside `enum AgentsAction`):
//
//     /// Delete an agent
//     #[command(visible_alias = "rm")]
//     Delete {
//         /// Agent ID
//         agent_id: String,
//     },
//
// REPLACE WITH:

/// Delete an agent (irreversible — cascades to conversations, attached KB
/// entries, and tool-dep edges)
#[command(visible_alias = "rm")]
Delete {
    /// Agent ID
    agent_id: String,

    /// Confirm deletion. Required because agent deletion is irreversible
    /// server-side. Matches the pattern used by `voices delete`,
    /// `dubbing delete`, `agents tools delete`, `phone batch delete`,
    /// and `phone whatsapp accounts delete`.
    #[arg(long)]
    yes: bool,
},

// ─── Done. No other cli.rs changes required from Fixer δ. ──────────────────
