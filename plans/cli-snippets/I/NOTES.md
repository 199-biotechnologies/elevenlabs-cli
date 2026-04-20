# Worker I — rich help

## Delivered

One new file: `src/help.rs` with 19 `pub const <NAME>_HELP: &str` constants
for clap's `after_long_help`. ASCII-safe, each <40 lines, no flag-description
duplication.

### Constants in `src/help.rs`

Spec P0 (high-value):
`TTS_HELP`, `STT_HELP`, `SFX_HELP`, `VOICES_LIBRARY_HELP`,
`VOICES_DESIGN_HELP`, `DIALOGUE_HELP`, `ALIGN_HELP`, `MUSIC_COMPOSE_HELP`,
`AGENTS_CREATE_HELP`, `AGENTS_ADD_KNOWLEDGE_HELP`, `PHONE_CALL_HELP`,
`DUBBING_CREATE_HELP`, `DICT_ADD_RULES_HELP`, `DOCTOR_HELP`,
`CONFIG_INIT_HELP`.

Also delivered (low-priority):
`UPDATE_HELP`, `SKILL_INSTALL_HELP`, `HISTORY_LIST_HELP`,
`USER_SUBSCRIPTION_HELP`.

Tips cover non-obvious pitfalls (first-colon split for IPA phonemes,
v0.2 add-knowledge PATCH fix, two-step voice design workflow, E.164
requirement, provider routing driven by phone record, env-shadow doctor
check, etc.). Examples all start with `$ elevenlabs ...`; at least one
per constant pipes into `jq` to reinforce JSON-on-pipe. Exit-code
references point at the 0/1/2/3/4 contract.

## Handoff files

- `cli-attach.md` — constant → clap struct/variant mapping table with the
  exact `#[command(after_long_help = ...)]` line.
- `mod.txt` — single line `pub mod help;` to add to `src/main.rs`.

## Open items

- Worker H's doctor shape (struct vs bare variant) unknown; attach table
  covers both.
- If Worker E renames `ComposeArgs` to `MusicComposeArgs`, the attach
  point moves; the constant is agnostic.

Compile-checked with `rustc --crate-type lib` — no syntax errors.
No new deps.
