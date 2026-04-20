# Worker E — music extensions

## Built

- Converted `src/commands/music.rs` into `src/commands/music/` with one
  submodule per user-facing action:
  - `compose.rs` — existing POST /v1/music (moved, unchanged behaviour)
  - `plan.rs` — existing POST /v1/music/plan (moved, unchanged)
  - `detailed.rs` — NEW POST /v1/music/detailed (audio + metadata split)
  - `stream.rs` — NEW POST /v1/music/stream (streaming write-to-disk)
  - `upload.rs` — NEW POST /v1/music/upload (multipart, returns song_id)
  - `stem.rs` — NEW POST /v1/music/stem-separation (multipart or song_id)
  - `video.rs` — NEW POST /v1/music/video-to-music (multipart video)
- Shared helpers in `mod.rs`: `build_compose_body` (covers compose,
  detailed, stream) and `stream_post_json_bytes` (chunked writer for the
  stream endpoint — inlines status-check so we don't have to change
  `client.rs`).
- Flagged all seven subcommands in `cli.rs` as proper `clap::Args`
  structs so the dispatch is a clean `match (args)` instead of a tuple
  unpack — easier to extend.

## Tests (new)

- `tests/music_endpoints.rs` — wiremock hit-path per new subcommand
  (detailed, stream, upload, stem-separation, video-to-music).
- `tests/music_detailed_split.rs` — verifies audio/metadata split, and
  that the default metadata path is `<audio>.metadata.json`.
- `tests/music_multipart.rs` — custom `Respond` impl asserts
  `multipart/form-data` content-type and that the expected part names and
  text fields appear in the body for `upload` and `video-to-music`.
- `tests/music_stream.rs` — 4 KB payload round-trip byte-for-byte through
  streaming, and 429 mapped to exit 4.
- Existing `tests/http_regression.rs` (compose + plan paths) still passes
  — CLI argv is unchanged for those two.

## Integration checklist (for the lead)

1. `src/cli.rs`: REPLACE the existing `MusicAction` enum and APPEND the
   six new `ComposeArgs` / `DetailedArgs` / `StreamArgs` / `UploadArgs` /
   `StemSeparationArgs` / `VideoToMusicArgs` structs per
   `plans/cli-snippets/E/cli.rs`.
2. `src/main.rs`: no change — existing
   `Commands::Music { action } => commands::music::dispatch(ctx, action).await`
   still routes correctly.
3. `src/commands/mod.rs`: no change — `pub mod music;` already there
   (now resolves to the directory).
4. `src/commands/agent_info.rs`: REPLACE the two `music.*` entries with
   the seven in `plans/cli-snippets/E/agent-info.json`.

## Dependencies

No new Cargo.toml entries. `futures-util` was already listed; I lean on
it for `.next()` in the stream helper.

## Open

- `music detailed` assumes the JSON response carries `audio_base64`
  (with `audio` as a graceful fallback). If the real API uses a
  different key once the endpoint ships, the fallback chain will catch
  it but we'll want a real payload to regression-lock.
- `music stem-separation <source>` disambiguates path vs song_id with a
  simple heuristic (contains `/`, `\`, `.`, or is an existing file →
  treat as file). If the API ever accepts song_ids that contain dots,
  we'll need a flag.
