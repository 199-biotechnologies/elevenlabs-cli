// No change needed in src/main.rs — the existing dispatch arm already
// routes to `commands::voices::dispatch(ctx, action)` regardless of which
// `VoicesAction` variant was parsed. When the lead wires in the new
// variants via cli.rs, the match inside `commands::voices::mod.rs`
// handles them.
//
// For reference, the relevant main.rs line is:
//     Commands::Voices { action } => commands::voices::dispatch(ctx, action).await,
