// ── Additions to `src/cli.rs` ──────────────────────────────────────────────
//
// 1. Add a new variant to `Commands` (under "Framework commands" section,
//    next to `AgentInfo`):

/// Run environment + dependency diagnostics
Doctor(DoctorArgs),

// 2. Add the args struct at the bottom of the file:

// ── Doctor ─────────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
pub struct DoctorArgs {
    /// Skip a named check (repeatable). Known names:
    /// config_file, api_key, env_shadow, api_key_scope, network,
    /// ffmpeg, disk_write, output_dir.
    #[arg(long = "skip", value_name = "NAME")]
    pub skip: Vec<String>,

    /// Timeout in milliseconds for the network reachability probe
    /// and API-scope probes (default 5000).
    #[arg(long = "timeout-ms", value_name = "MS", default_value_t = 5000)]
    pub timeout_ms: u64,
}
