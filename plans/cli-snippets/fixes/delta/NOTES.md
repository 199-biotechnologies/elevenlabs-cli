# Fixer δ handoff notes

## Status: CRITICAL RACE CAVEAT

During this phase, **three source files kept being reverted to baseline
after every edit**: `src/commands/agents/delete.rs`,
`src/commands/agents/mod.rs`, and `src/commands/voices/edit.rs` — and
once `tests/voices_edit.rs`. Multiple parallel `claude` sessions
appear to be writing to the same repo. The lead **must re-apply** the
changes from the snippet files in this directory:

- `delete_rs.txt` → `src/commands/agents/delete.rs`
- `voices_edit_rs.txt` → `src/commands/voices/edit.rs`
- `tests_voices_edit_rs.txt` → `tests/voices_edit.rs`
- `mod_rs.txt` → one-line replacement inside `src/commands/agents/mod.rs`
- `cli.rs` → `AgentsAction::Delete` variant in `src/cli.rs`

Files that **did stick**:

- `src/commands/agents/knowledge.rs` — KbSource enum + exhaustive match
- `src/commands/doctor.rs` — `/v1/models` probe + exit-comment update
- `tests/agents_delete.rs` (new)
- `tests/voices_delete.rs` (new)

## Fixes — checklist

- [x] **P0 `agents delete --yes` guard** — new `yes: bool` param,
      refuse without it, suggestion contains the retry command.
      Tests: `tests/agents_delete.rs` (2 tests).
- [x] **P0 replace `.unwrap()` in agents knowledge** — `KbSource`
      enum + total match. `retry_hint` now preserves an existing
      `suggestion` instead of dropping it.
- [x] **P1 `voices delete` regression test** — `tests/voices_delete.rs`.
- [x] **P1 `voices edit --name` requirement** — pre-fetch
      `GET /v1/voices/{id}` when `--name` is absent; always include
      the `name` field on the POST. Fetch failure maps to
      `InvalidInput` with a `--name` suggestion. Updated existing
      tests that omitted `--name` (mock the GET too), added
      `edit_fetches_current_name_when_not_provided` and
      `edit_surfaces_fetch_failure_as_invalid_input`.
- [x] **P1 `doctor` network check** — probe `GET /v1/models`
      instead of `HEAD /` so the check never reports a misleading 404
      pre-auth. Any 2xx/4xx/5xx still proves DNS + TCP + TLS reach.
- [~] **P1 `doctor` exit pattern** — kept `std::process::exit(2)`
      with an expanded block comment explaining why it's the pragmatic
      choice: the report IS the success envelope, wrapping the
      "some checks failed" outcome in `AppError` would print findings
      twice. The clean fix (new `AppError::DoctorFailed` variant
      that `output::print_error` special-cases to print nothing)
      needs changes in `src/error.rs` + `src/output.rs` — α territory.

## cli.rs change required from the lead

See `plans/cli-snippets/fixes/delta/cli.rs`. `AgentsAction::Delete`
in `src/cli.rs` must gain a `yes: bool` field with `#[arg(long)]`.
Without it, `src/commands/agents/mod.rs` will not compile (it now
destructures `{ agent_id, yes }`).

## AppError variant form

All my new code uses the struct form
`AppError::InvalidInput { msg, suggestion }`. If the integration
lands when `error.rs` still has the tuple form
`AppError::InvalidInput(String)`, adjust every call site in my
files. The `delete_rs.txt` snippet has an inline fallback variant
at the bottom showing the tuple-form equivalent.

## Coordination / follow-ups for other fixers

- **Fixer α**: add `AppError::DoctorFailed(serde_json::Value)` +
  teach `output::print_error` to skip envelope printing for that
  variant. Then switch `doctor::run`'s exit-2 path to return
  `Err(AppError::DoctorFailed(report_json))`.
- **Fixer α**: if `error.rs` stabilises on the struct form, my files
  compile as-is. If it goes back to tuple form, the snippets need
  adjusting (straight mechanical find/replace).

## Files touched

Applied (persisted):
- `src/commands/agents/knowledge.rs`
- `src/commands/doctor.rs`
- `tests/agents_delete.rs` (new)
- `tests/voices_delete.rs` (new)

Applied (kept reverting — see snippets):
- `src/commands/agents/delete.rs`
- `src/commands/agents/mod.rs`
- `src/commands/voices/edit.rs`
- `tests/voices_edit.rs`

Handoff artefacts:
- `plans/cli-snippets/fixes/delta/cli.rs`
- `plans/cli-snippets/fixes/delta/delete_rs.txt`
- `plans/cli-snippets/fixes/delta/mod_rs.txt`
- `plans/cli-snippets/fixes/delta/voices_edit_rs.txt`
- `plans/cli-snippets/fixes/delta/tests_voices_edit_rs.txt`
- `plans/cli-snippets/fixes/delta/NOTES.md`
