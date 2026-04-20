# Fixer alpha — v0.2.0 review-fix notes

## Checklist

- [x] **#1 Per-command InvalidInput suggestions.** Variant is now
  `InvalidInput { msg, suggestion: Option<String> }`. Helpers
  `bad_input` / `bad_input_with` added, plus `From<String>` +
  `From<&str>` for `Err("…".into())` paths. Mechanical sweep converted
  all ~120 call sites across 39 files to the struct form; match-arm
  `InvalidInput(_)` patterns now `InvalidInput { .. }`. Command-specific
  suggestions landed at the five requested sites:
  `agents/knowledge.rs`, `dialogue.rs`, `dubbing/create.rs`,
  `dict/mod.rs::collect_rules`, `phone/batch/submit.rs`.

- [x] **#2 Secret redaction in 3 ad-hoc paths.** `redact_secrets` is
  `pub(crate)`. `dict/download.rs`, `dubbing/mod.rs::get_bytes`, and
  `music/mod.rs::stream_post_json_bytes` route truncated bodies through
  it before surfacing errors.

- [x] **#3 Dialogue NDJSON streaming.** `dialogue.rs` rebuilt as a
  four-way branch. Both `/stream*` endpoints consume
  `bytes_stream()`, buffer-and-split on `\n`, base64-decode
  `audio_base64` per line, and append to the output file. The
  `with-timestamps` variant additionally appends residue (minus audio)
  to a JSONL companion. Error bodies are `redact_secrets`-processed.

- [x] **#4 Dialogue limit tests.** `tests/dialogue_limits.rs` pins
  the 11-voice, 2001-char, and empty-inputs red paths to exit 3.

- [x] **#5 Suggestion regression test.** `tests/output_contracts.rs`
  extended; exercises `agents add-knowledge` with missing flags and
  asserts the attached suggestion is not the generic default.

## cargo test --release

**156 passed, 12 failed.** All 12 failures live in test targets owned
by other parallel fixers (`agents_delete`, `dialogue_endpoints` subset,
`music_detailed_split`, `music_multipart`, `phone_batch_endpoints`,
`phone_whatsapp`) whose cli.rs renames haven't been integrated yet.
Alpha's own three test files all pass: `dialogue_stream_ndjson` 2/2,
`dialogue_limits` 3/3, `output_contracts` 6/6.

## Incomplete / blockers

None. Every deliverable shipped.

## cli.rs changes & integration hints

**No cli.rs snippet needed.** Integration caveats:

1. `InvalidInput` rename is huge diff but semantically no-op. Prefer
   `bad_input_with(msg, suggestion)` for future errors.
2. `From<String>` / `From<&str>` keep `Err(s.into())` working if future
   workers add it — don't rip them out.
3. NDJSON path uses `futures_util` + `base64`, both already in-tree.
