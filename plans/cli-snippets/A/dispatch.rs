// plans/cli-snippets/A/dispatch.rs
// ─────────────────────────────────────────────────────────────────────────────
// No dispatch changes needed for src/main.rs.
// The existing line
//
//     Commands::Agents { action } => commands::agents::dispatch(ctx, action).await,
//
// continues to work. `commands::agents` is now a directory with `mod.rs`
// exposing `dispatch` with the same signature. The new `AgentsAction::Update`,
// `::Duplicate`, and `::Tools { action }` variants are handled inside
// `commands::agents::dispatch` rather than in main.rs.
// ─────────────────────────────────────────────────────────────────────────────
