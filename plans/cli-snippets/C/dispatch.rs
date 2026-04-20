// === Worker C dispatch ===
// Add to the big match in src/main.rs (under "Domain commands") and the
// import list at the top.
//
// Imports: `Commands::Dubbing` lands on the same path you already use for
// other domain commands; nothing new to import. `DubbingAction` and
// `DubbingResourceAction` are only referenced inside commands::dubbing.

            Commands::Dubbing { action } => commands::dubbing::dispatch(ctx, action).await,
