# Worker H — `doctor`

## Deliverable checklist

- [x] `src/commands/doctor.rs` — 8 checks (`config_file`, `api_key`,
      `env_shadow`, `api_key_scope`, `network`, `ffmpeg`, `disk_write`,
      `output_dir`), `--skip <name>` repeatable, `--timeout-ms <n>`
      (default 5000).
- [x] Output via `output::print_success_or` (JSON envelope + coloured
      comfy-table human view with per-fail suggestion lines).
- [x] Exit semantics: pass/warn → 0, any fail → 2 (via
      `std::process::exit(2)` after the report is printed).
- [x] Network errors inside checks become `fail` results — never
      bubble as `AppError`. Every `fail` has a concrete suggestion.
- [x] API-scope probe hits `/v1/user` and `/v1/voices` independently
      → restricted-key warning when voices ok but user 401/403.
- [x] Env-shadow check uses `AuthKeyState::env_ignored_by_file()` to
      catch the v0.1.5 bug class.
- [x] Network check honours `ELEVENLABS_API_BASE_URL` and notes it.

## Tests written

- `tests/doctor_env_shadow.rs` — warn on mismatch, pass on match, pass
  on env-only.
- `tests/doctor_restricted_key.rs` — 401 /v1/user + 200 /v1/voices →
  warn; both 401 → fail (exit 2).
- `tests/doctor_all_pass.rs` — envelope shape (`{checks, summary}`,
  `{name, status, detail, suggestion}` per check), `--skip` behaviour,
  missing-key → fail + exit 2.

## Integration checklist (for lead)

1. Append `plans/cli-snippets/H/cli.rs` additions to `src/cli.rs`:
   - `Commands::Doctor(DoctorArgs)` variant.
   - `DoctorArgs` struct at the bottom.
2. Append `plans/cli-snippets/H/dispatch.rs` match arm to `src/main.rs`
   inside the `match cli.command` block.
3. Append `plans/cli-snippets/H/mod.txt` (`pub mod doctor;`) to
   `src/commands/mod.rs` (alphabetical slot between `dict` and
   `dubbing`, or after `update` — either works).
4. Merge `plans/cli-snippets/H/agent-info.json` entry into the
   `commands` map in `src/commands/agent_info.rs`.
5. `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test`.
6. Add an `agent-info` routability assertion in
   `tests/agent_info_contract.rs` (e.g. `commands.contains_key("doctor")`
   + `bin().arg("doctor").args(["--skip", …all…]).assert().code(0);`).

## Dependencies

No new Cargo deps — uses `serde`, `serde_json`, `reqwest`, `tokio`,
`toml`, `owo-colors`, `comfy-table`, all already in `Cargo.toml`.

## Open items / notes

- Could NOT run `cargo check` to verify: the workspace is mid-
  integration (other workers' dir-based modules reference clap types
  the lead hasn't wired yet, so `cargo check` fails with 28 errors
  unrelated to this worker). Ran a careful pass instead. The module
  stands alone (no internal deps on other workers' work).
- `run()` uses `std::process::exit(2)` after printing the success
  envelope so the user sees the full report and a non-zero exit, not a
  duplicated error envelope. An alternative is to return an `AppError`
  variant that main treats specially; left as-is because `exit(2)` is
  the pattern `config check` effectively uses and keeps the report
  clean.
