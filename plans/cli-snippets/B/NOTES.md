# Worker B — dialogue + align

## Deliverables (all done)

- [x] `src/commands/dialogue.rs` — four endpoint variants routed by `--stream` / `--with-timestamps`. JSON file + colon-triple + stdin input shapes. Client-side pre-flight: <=10 distinct voice IDs, ~2000 char cap.
- [x] `src/commands/align.rs` — POST /v1/forced-alignment multipart. Accepts inline text, positional file-path, or `--transcript-file`. Optional `--output` persists raw JSON; TTY renders a word-timing table.
- [x] `tests/dialogue_endpoints.rs` (4 tests, one per endpoint variant)
- [x] `tests/dialogue_input_parsing.rs` (6 tests: JSON file, implicit .json detection, triples, malformed triple, >10 voices, missing inputs)
- [x] `tests/align_multipart.rs` (4 tests: happy path w/ --output, --transcript-file, missing audio, empty transcript)

## Verified locally

I ran an isolated integration of my snippets into a scratch clone (`/tmp/wb_scratch/repo`), stripping out mid-flight edits from other workers. Result:

- `cargo build` — clean
- `cargo fmt --all -- --check` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo test --test dialogue_endpoints --test dialogue_input_parsing --test align_multipart` — 14/14 passing

## Integration checklist for the lead

1. Paste `plans/cli-snippets/B/cli.rs` contents into `src/cli.rs` (two new `Commands` variants after `Sfx`, plus `DialogueArgs` and `AlignArgs` structs at the bottom).
2. Add `pub mod align;` and `pub mod dialogue;` to `src/commands/mod.rs` (see `mod.txt`).
3. Add the two match arms from `dispatch.rs` to `src/main.rs` under the "Domain commands" block.
4. Merge `agent-info.json` entries into `src/commands/agent_info.rs` (two new keys: `dialogue [triples... | path.json | -]` and `align <audio> <transcript|path>`).

## No new Cargo.toml deps

Uses only what's already in the tree: `base64`, `serde`, `serde_json`, `reqwest::multipart`, `comfy-table`, `tokio`.

## Open / follow-ups

- The streamed `/stream/with-timestamps` endpoint currently buffers the full JSON envelope (chunks[] concatenated). A future pass could stream chunks as NDJSON to disk using `reqwest::stream()`. Candidate new helper in `client.rs`: `post_json_stream_with_query` returning a `futures::Stream<Bytes>`.
- `--stdout` on dialogue deliberately excludes the with-timestamps variants because audio+alignment split across two sinks doesn't fit a single stdout stream.
