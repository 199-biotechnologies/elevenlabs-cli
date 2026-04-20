# Worker C — Dubbing

## Deliverable checklist

Modules in `src/commands/dubbing/`:
- [x] `mod.rs` — enum dispatch + shared `load_patch_body` + `get_bytes` helpers
- [x] `create.rs` — POST /v1/dubbing (multipart, file OR source_url)
- [x] `list.rs` — GET /v1/dubbing with filters
- [x] `show.rs` — GET /v1/dubbing/{id}
- [x] `delete.rs` — DELETE /v1/dubbing/{id} (`--yes` required)
- [x] `audio.rs` — GET /v1/dubbing/{id}/audio/{lang} (bytes download)
- [x] `transcript.rs` — GET /v1/dubbing/{id}/transcript/{lang}/format/{fmt} (new 2026 endpoint, `--format srt|webvtt|json`)
- [x] `resource/mod.rs` + 5 files: `transcribe.rs`, `translate.rs`, `dub.rs`, `render.rs`, `migrate.rs` (all POST `/v1/dubbing/resource/{id}/…`, optional `--patch <json_file>`)

Each file stays under 200 lines.

## Tests

`tests/dubbing_endpoints.rs` — method+path routing for create (URL), list, show, delete (`--yes` accepted AND refused without it), get-audio, and all 5 resource POSTs.

`tests/dubbing_multipart.rs` — create builds the expected multipart body with `source_url` (no file field) and with `--file` (no source_url field); also covers the "no source" error path.

`tests/dubbing_transcript_formats.rs` — `--format` srt/webvtt/json each route to the `/format/{fmt}` path, and an unknown format exits 3 via clap.

## Framework compliance

- Output via `output::print_success_or`.
- Errors are `AppError` variants — every one has a `suggestion` via the shared mapping in `src/error.rs`.
- `delete` refuses without `--yes` → exits 3 with `invalid_input`.
- `create` requires exactly one of `--file` / `--source-url`, errors out with `invalid_input` if neither.
- No stdin prompts. No new crates. No changes outside ownership tree.

## Client helpers

Reused `client.post_multipart_json`, `client.post_json`, `client.get_json`, `client.get_json_with_query`, and `client.delete`. Needed a GET-that-returns-bytes helper which does NOT exist on `ElevenLabsClient` — inlined as `super::get_bytes` inside `src/commands/dubbing/mod.rs`. It mirrors the non-success branch of `client::check_status` (private) so auth/rate/api mapping stays consistent.

**Optional follow-up for the lead:** promote `get_bytes` to a first-class `ElevenLabsClient::get_bytes` method in `src/client.rs` and have `dict`/dubbing reuse it — I didn't do it here because `src/client.rs` is outside my ownership tree.

## Integration checklist for the lead

1. Paste the `Commands::Dubbing` variant + `DubbingAction`, `DubbingCreateArgs`, `DubbingResourceAction` enums from `cli.rs` into `src/cli.rs`.
2. Add the `Commands::Dubbing { action } => commands::dubbing::dispatch(ctx, action).await,` match arm to `src/main.rs`.
3. Append `pub mod dubbing;` from `mod.txt` to `src/commands/mod.rs`.
4. Merge all 11 entries from `agent-info.json` into the `commands` map in `src/commands/agent_info.rs`.
5. Run `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test` — the three `dubbing_*.rs` suites must pass on wiremock.

## Dependencies

None added. Uses only `reqwest::multipart::{Form, Part}`, `serde_json`, `tokio`, `bytes::Bytes` — all already in `Cargo.toml`.

## Open questions

- The `list` endpoint response key: I parsed `dubs` per the SDK. If the server rolls a breaking rename (e.g. `dubbings`), the human table will show `(no dubs)` even when data is present — the JSON envelope still carries raw `data` so agents see the truth.
- `watermark`, `highest_resolution`, `drop_background_audio`, `use_profanity_filter`, `dubbing_studio`, `disable_voice_cloning` accept `--flag=true|false` rather than being store_true. This is on purpose: the API distinguishes "unset" from "false" for several of these, and we don't want our CLI to silently flip an account default.
