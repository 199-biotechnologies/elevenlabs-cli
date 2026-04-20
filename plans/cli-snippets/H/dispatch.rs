// ── Additions to `src/main.rs` ─────────────────────────────────────────────
//
// 1. Import `DoctorArgs` alongside the other args types if you use the
//    shortened form. (The existing main.rs uses `cli::{Cli, Commands, …}`;
//    you can pattern-match on `cli::DoctorArgs` inline or add it to the
//    existing `use cli::{…}` list.)
//
// 2. Add the match arm in the domain-commands section of the big `match
//    cli.command` block — anywhere among the framework commands works;
//    I'd put it next to `Commands::Update`:

Commands::Doctor(args) => commands::doctor::run(
    ctx,
    commands::doctor::DoctorOptions {
        skip: args.skip,
        timeout_ms: args.timeout_ms,
    },
)
.await,
