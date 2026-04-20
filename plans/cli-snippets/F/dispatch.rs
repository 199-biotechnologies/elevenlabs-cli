// No changes required — `Commands::Phone { action }` already routes to
// `commands::phone::dispatch(ctx, action)` in `src/main.rs`, and the new
// sub-action enums are dispatched inside `src/commands/phone/mod.rs`.
//
// For completeness, the match arm that must remain in main.rs is:
//
//     Commands::Phone { action } => commands::phone::dispatch(ctx, action).await,
