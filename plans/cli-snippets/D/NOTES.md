# Worker D — Pronunciation Dictionaries (`dict`)

## Deliverables (all landed)

- [x] `src/commands/dict/mod.rs` — `dispatch` + shared `parse_rule` / `parse_alias_rule` / `collect_rules` helpers.
- [x] `src/commands/dict/list.rs` — `GET /v1/pronunciation-dictionaries` with `--cursor` / `--page-size` / `--search`.
- [x] `src/commands/dict/add_file.rs` — multipart `POST /add-from-file` (file, name, description, workspace_access).
- [x] `src/commands/dict/add_rules.rs` — `POST /add-from-rules` from `--rule` / `--alias-rule` flags.
- [x] `src/commands/dict/show.rs` — `GET /v1/pronunciation-dictionaries/{id}`.
- [x] `src/commands/dict/update.rs` — `PATCH` with `--name` / `--description` / `--archive` (archive is reversible server-side → no `--yes` required, surfaced in `--help`).
- [x] `src/commands/dict/download.rs` — `GET /{id}/{version}/download` → writes PLS XML bytes to disk; auto-resolves latest version when `--version` is omitted.
- [x] `src/commands/dict/set_rules.rs` — `POST /{id}/set-rules` with `--case-sensitive` / `--word-boundaries` (both `Option<bool>` so callers can explicitly unset).
- [x] `src/commands/dict/add_rules_to.rs` — `POST /{id}/add-rules`.
- [x] `src/commands/dict/remove_rules.rs` — `POST /{id}/remove-rules` with repeatable `--word`.

## Tests (all in owned tree)

- `tests/dict_endpoints.rs` — routing + method smoke tests for every subcommand, plus empty-patch / missing-word exit-3 coverage.
- `tests/dict_rule_parsing.rs` — `parse_rule` contract exercised through `add-rules`: phoneme/alias dispatch, first-`:`-only split for IPA colons, whitespace trimming, empty word/phoneme/alias → exit 3, no-rules → exit 3.
- `tests/dict_xml_download.rs` — explicit-version download writes raw PLS bytes, auto-version path first fetches `latest_version_id`, 404s surface as `api_error` envelope.

## Handoff snippets

- `cli.rs` — `Commands::Dict { action: DictAction }` variant plus the `DictAction` enum (nine subcommands).
- `dispatch.rs` — one `match` arm for `src/main.rs`.
- `agent-info.json` — manifest entries for all nine subcommands.
- `mod.txt` — `pub mod dict;`.

## Integration checklist for lead

1. Append the `Commands::Dict { … }` variant to `src/cli.rs` and paste the `DictAction` enum at the bottom.
2. Append the dispatch arm in `src/main.rs`.
3. Append `pub mod dict;` to `src/commands/mod.rs` (alphabetical).
4. Merge `agent-info.json` entries into `src/commands/agent_info.rs` `commands` map.
5. Run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.

## Notes

- No new Cargo dependencies needed — reuses `reqwest`, `serde_json`, `tokio::fs`, `comfy-table`, `owo-colors`.
- `download.rs` reuses the existing `ElevenLabsClient.http` for a raw GET (same pattern as `doctor.rs:572` and `dubbing/mod.rs:134`).
- `parse_rule` splits on the first `:` only so IPA length marks / tone colons later in the phoneme survive — regression test locks this in.
