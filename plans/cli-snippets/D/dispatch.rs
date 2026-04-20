// ─── dispatch.rs additions for Worker D ───────────────────────────────────
//
// Append to the `match cli.command { … }` block in `src/main.rs`, alongside
// the other domain commands:

Commands::Dict { action } => commands::dict::dispatch(ctx, action).await,
