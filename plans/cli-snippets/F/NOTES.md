# Worker F — phone batch + whatsapp

## Deliverables (all implemented)

- [x] `phone.rs` deleted; converted to `src/commands/phone/` directory module.
- [x] `phone list` + `phone call` preserved as `phone/list.rs` + `phone/call.rs`.
- [x] `phone batch submit --agent --phone-number --recipients [--name --scheduled-time-unix]` with CSV+JSON parser + stdin (`-`) support.
- [x] `phone batch list [--page-size --cursor --status --agent-id]`.
- [x] `phone batch show <id>` (per-call status surfaced via raw pretty-print).
- [x] `phone batch cancel <id>` (reversible via retry, no `--yes` gate — per spec).
- [x] `phone batch retry <id>`.
- [x] `phone batch delete <id> --yes` (gated, matches `voices delete`/`dubbing delete` pattern).
- [x] `phone whatsapp call --agent --whatsapp-account --recipient`.
- [x] `phone whatsapp message --agent --whatsapp-account --recipient [--text | --template]` (mutually exclusive via clap + runtime guard).
- [x] `phone whatsapp accounts {list,show,update --patch <json>,delete --yes}`.

## Tests

- `tests/phone_batch_submit.rs` — CSV parse, JSON parse, body shape, missing-file rejection, bad-JSON-column rejection (4 tests).
- `tests/phone_batch_endpoints.rs` — list/show/cancel/retry + delete w/ and w/o `--yes` (6 tests).
- `tests/phone_whatsapp.rs` — call/message/accounts CRUD + mutual-exclusion guards (9 tests).
- Plus 7 in-module unit tests for the CSV parser inside `phone/batch/submit.rs`.

## Integration checklist (for lead)

1. Replace `PhoneAction` in `src/cli.rs` with the version in `cli.rs`; append `PhoneBatchAction`, `PhoneWhatsappAction`, `PhoneWhatsappAccountsAction`.
2. `src/main.rs` match arm unchanged — `Commands::Phone { action } => commands::phone::dispatch(ctx, action).await,` already routes correctly.
3. `src/commands/mod.rs` unchanged — `pub mod phone;` now resolves to the new directory module.
4. Append the JSON fragments in `agent-info.json` to the `commands` map of `src/commands/agent_info.rs` (replacing the existing `phone list`/`phone call` entries).

## Dependency requests

**None.** The CSV parser is hand-rolled (roughly 60 lines of RFC-4180 subset) — no new Cargo.toml entries required. Chose not to pull in the `csv` crate because (a) we only handle one file shape with two columns, (b) the existing dep surface is intentionally small per CLAUDE.md.

## Open questions

- API: `GET /v1/convai/batch-calling/workspace` response envelope shape. The list module tolerates `{batch_calls: []}`, `{batches: []}`, or a bare array — pick whichever the live API returns (no change needed).
- API: `show` returns the full batch with per-call status; we pretty-print JSON verbatim rather than table-formatting per-call rows (matches `agents show`).
- The `phone batch cancel`/`retry` endpoints accept an empty POST body `{}` per the docs; if the live API requires a specific shape, update `cancel.rs`/`retry.rs` (localised change).

Contributed by Claude Code (claude-opus-4-7, 1M ctx).
