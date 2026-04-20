// ── Add to the main match in src/main.rs under the "Domain commands" block ───

Commands::Dialogue(args) => commands::dialogue::run(ctx, args).await,
Commands::Align(args) => commands::align::run(ctx, args).await,
