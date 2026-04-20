# Fixer β — music endpoint contract fixes

All four SDK contracts verified against `elevenlabs-python/src/elevenlabs/music/raw_client.py`.

## Checklist

- [x] **P0 music detailed** — POST `/v1/music/detailed` now parses the `multipart/mixed` response (JSON metadata part + binary audio part). Hand-rolled boundary splitter (no new deps). Audio → `--output`, JSON → `--save-metadata` (default `<output>.metadata.json`). Sends `Accept: multipart/mixed`.
- [x] **P0 music stem-separation** — POST `/v1/music/stem-separation` now sends multipart with only the `file` part (no `song_id` branch, no `stems` field). New flags `--output-format`, `--stem-variation-id`, `--sign-with-c2pa`. Response is a ZIP; extracted into `--output-dir` via the `zip` crate.
- [x] **P1 music upload** — POST `/v1/music/upload` now sends only `file` + optional `extract_composition_plan` bool. Dropped `--name` and `--composition-plan`.
- [x] **P1 music video-to-music** — multipart part renamed `file` → `videos` (SDK contract). Dropped `--model`. Added `--sign-with-c2pa` (form field) and kept `--output-format` as query.

## Deps requested (lead must add)

```toml
# Cargo.toml [dependencies]
zip = { version = "2", default-features = false, features = ["deflate"] }
```

Used by `src/commands/music/stem.rs` (runtime unzip of the stem-separation response) and by `tests/music_endpoints.rs` (fabricating a fake ZIP for the wiremock). No runtime alternative without a new crate.

## cli.rs changes

Full struct replacements in `plans/cli-snippets/fixes/beta/cli.rs`. Summary:

| Struct | Drop | Add | Rename |
|---|---|---|---|
| `UploadArgs` | `name`, `composition_plan` | `extract_composition_plan` | — |
| `StemSeparationArgs` | `stems: Vec<String>` default list | `output_format`, `stem_variation_id`, `sign_with_c2pa` | `source: String` → `file: String` |
| `VideoToMusicArgs` | `model` | `sign_with_c2pa` | — |
| `DetailedArgs` | — | — | — (flag surface unchanged; only HTTP contract changed) |

CHANGELOG note needed: “Breaking: `music upload --name`/`--composition-plan` removed; `music stem-separation` arg renamed `SOURCE` → `FILE` and `--stems` dropped; `music video-to-music --model` removed.”

## Test updates

- `tests/music_detailed_split.rs` — serves a fabricated `multipart/mixed` response, asserts audio file = binary part (not base64), metadata file = JSON part.
- `tests/music_endpoints.rs` — `music_detailed` test now serves multipart/mixed; `music_stem_separation` now serves a zip archive with two stems and asserts the unzipped files; `music_upload` no longer passes `--name`; `music_video_to_music` unchanged.
- `tests/music_multipart.rs` — upload assertion checks for `name="file"` + `name="extract_composition_plan"` and forbids `name="name"`/`name="composition_plan"`; video-to-music assertion checks for `name="videos"` (not `name="file"`) and forbids `name="model_id"`.
- `tests/music_stream.rs` — untouched.

## Coordination warnings

- **My writes kept being reverted during this session.** Another worker (probably Fixer α running a global `AppError::InvalidInput` migration) was overwriting `src/commands/music/{detailed,stem,upload,video}.rs` mid-stream. The lead should verify that the v0.2 music command files actually end in the β state documented here — a diff that still shows `InvalidInput { msg: ..., suggestion: None }` in these four files means α's last sweep won the race and my fixes must be re-applied from this NOTES.md.
- When the struct-form errors land across the tree, these four β files should be using `AppError::bad_input(...)` / `AppError::bad_input_with(...)` helpers from `src/error.rs`. No `InvalidInput { msg: ..., suggestion: ... }` literals in β's files.

## Verified against SDK

Python SDK (commit on `main` as of the session): `src/elevenlabs/music/raw_client.py`.

- `compose_detailed` → `v1/music/detailed`, returns `multipart/mixed` streaming (JSON metadata + binary audio). Confirmed.
- `separate_stems` → `v1/music/stem-separation`, multipart `file` + form `stem_variation_id` + form `sign_with_c2pa`, query `output_format`, returns ZIP archive. Confirmed.
- `upload` → `v1/music/upload`, multipart `file` + form `extract_composition_plan`, returns `MusicUploadResponse` JSON. Confirmed.
- `video_to_music` → `v1/music/video-to-music`, multipart `videos: List[File]` + form `description` + form `tags: List[str]` + form `sign_with_c2pa`, query `output_format`, returns audio bytes. Confirmed.

All contracts matched. No SDK details left unverified.
