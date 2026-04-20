# Worker G — voices extensions

## Deliverables

- Converted `src/commands/voices.rs` (single file) to `src/commands/voices/`
  directory with one submodule per subcommand (`list`, `show`, `search`,
  `library`, `clone`, `design`, `save_preview`, `delete`, `add_shared`,
  `similar`, `edit`, plus shared `resolve`).
- New subcommands:
  - `voices add-shared <public_user_id> <voice_id> --name <new_name>
    [--bookmarked <bool>]` → POST `/v1/voices/add/{public}/{voice_id}`.
  - `voices similar <audio_file>` → POST `/v1/similar-voices` multipart.
    Exposes official `--similarity-threshold`, `--top-k` plus prompt-required
    form-field filters (`--gender/--age/--accent/--language/--use-case`).
  - `voices edit <voice_id>` with `--name/--description/--labels/--add-sample/
    --remove-sample/--remove-background-noise`. Multipart POST to
    `/v1/voices/{id}/edit`; sample removals fan out to `DELETE
    /v1/voices/{id}/samples/{sample_id}`.
- `voices list` extended with Apr 13 2026 v2 flags: `--next-page-token`,
  `--voice-type` (incl. `non-community`), `--category`, `--fine-tuning-state`,
  `--collection-id`, `--include-total-count`, `--voice-id` (repeatable).
- `voices show` surfaces Apr 7 2026 additions (`is_bookmarked`,
  `recording_quality`, `labelling_status`, `recording_quality_reason`, labels)
  in the human TTY table; JSON passthrough unchanged.

## Tests

- `tests/voices_add_shared.rs` (path interpolation, body, `--bookmarked`,
  missing `--name` exits 3).
- `tests/voices_similar_multipart.rs` (endpoint, filter form fields, missing
  file exits 3).
- `tests/voices_edit.rs` (rename, labels→JSON form field, remove-sample DELETE
  fan-out, no-op exits 3, bad `--labels` format exits 3).
- `tests/voices_list_v2_flags.rs` (voice_type, filter bundle, repeated
  voice_ids).

## Integration checklist (for the lead)

1. Replace `pub enum VoicesAction` in `src/cli.rs` with
   `plans/cli-snippets/G/cli.rs`.
2. Append agent-info entries from `plans/cli-snippets/G/agent-info.json` to
   `src/commands/agent_info.rs`.
3. `main.rs` + `commands/mod.rs` unchanged.
4. No new Cargo deps.

## Follow-up for lead

Both `src/commands/tts.rs` and `src/commands/audio.rs` duplicate the voice
name-resolver logic. `voices::resolve_voice_id_by_name` is already exported
from the new module; a follow-up PR can drop both inline copies and call the
shared helper.
