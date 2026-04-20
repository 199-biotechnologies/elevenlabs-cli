# Technical Review: elevenlabs-cli v0.2.0

## 1. Architectural Consistency & Rough Edges

### [P1] Manual Exit in `doctor.rs` (src/commands/doctor.rs:81)
The `doctor` command calls `std::process::exit(2)` directly to avoid the "double error envelope" when a check fails. This bypasses the framework's standard `AppError` reporting path and makes the command harder to test in-process. 
**Fix:** Introduce a `SilentError(i32)` variant in `AppError` that maps to an exit code but suppresses the standard JSON/Human error envelope, or allow `run()` to return a result that the main loop handles without re-printing the report.

### [P1] Duplicated Error Mapping in `dubbing` (src/commands/dubbing/mod.rs:89)
The `get_bytes` helper in the dubbing module manually drives `reqwest` and re-implements status code mapping (401, 403, 429). This logic already exists in `ElevenLabsClient` and `AppError`.
**Fix:** Move `get_bytes` into `ElevenLabsClient` as a first-class method alongside `post_json_bytes` to ensure consistent auth/rate-limit handling across all binary downloads.

### [P2] Redundant Option Mirroring (src/commands/doctor.rs:32)
`doctor.rs` defines a private `DoctorOptions` struct that mirrors `DoctorArgs` from `cli.rs`. While this decouples the implementation from the CLI parser, it adds boilerplate to every new flag.
**Fix:** Pass `DoctorArgs` directly unless the command is intended to be used as a library component by other modules.

---

## 2. Test Coverage Gaps

### [P1] Missing Validation Limit Tests (src/commands/dialogue.rs:56-72)
The `dialogue` command enforces a 10-voice limit and a 2,000-character limit. However, the existing suites (`dialogue_endpoints.rs`, `dialogue_input_parsing.rs`) do not appear to have "red-path" tests that verify these specific limits trigger an `AppError::InvalidInput` (Exit 3).
**Fix:** Add `tests/dialogue_limits.rs` to verify client-side pre-flights.

### [P1] Multipart Body Shape Verification (src/commands/align.rs:75)
Alignment and dubbing creation use multipart forms. While success paths are tested, there are no tests for:
1. Missing `file` or `text` parts (server-side 400 mapping).
2. Invalid MIME type resolution for obscure audio extensions.
3. Spooled file behavior on large inputs (src/commands/align.rs:80).

### [P2] Mid-Stream Error Resilience (src/commands/music/mod.rs:136)
`stream_post_json_bytes` writes to disk as chunks arrive. If the connection drops or the server sends an error after the first few bytes, the current implementation may leave a partial/corrupted file without a clear error state to the user.
**Fix:** Add a test case in `music_stream.rs` that simulates a truncated response body.

---

## 3. Ergonomics & CLI Structure

### [P0] Unwieldy `src/cli.rs` (src/cli.rs:1)
At 2,053 lines, `cli.rs` is a bottleneck. Adding a single flag requires navigating a massive file.
**Fix:** Split into a `src/cli/` directory:
- `mod.rs`: Main `Cli` and `Commands` enum.
- `speech.rs`: `TtsArgs`, `SttArgs`, `SfxArgs`, `DialogueArgs`, `AlignArgs`.
- `voices.rs`: `VoicesAction`.
- `phone.rs`: `PhoneAction`.
- `music.rs`, `dubbing.rs`, `agents.rs`, etc.

### [P1] Positional vs. File Resolution Heuristic (src/commands/align.rs:109)
`align.rs` uses a heuristic (length < 512, no newlines, exists on disk) to decide if the second positional is a transcript string or a file path. `dialogue.rs` has a similar but different check (`looks_like_json_file`).
**Fix:** Extract a `resolve_input_text(arg: &str)` helper to `src/commands/mod.rs` to ensure consistent "magic" path detection across the whole CLI.

---

## 4. Help Content Evaluation (src/help.rs)

**Verdict:** **High Utility / Retain.**
The `after_long_help` content is excellent. 
- **Signal:** High. It focuses on the "Two-Step workflow" for Voice Design and the "Plan-first" workflow for Music, which are not obvious from flag names alone.
- **Format:** The `EXAMPLES` block uses consistent `$` prefixing and `elevenlabs` binary names, making them perfectly digestible for AI sub-agents and humans.
- **Recommendation:** No changes needed to content; just ensure the split of `cli.rs` maintains the links to these constants.

---

## 5. Summary Table

| Finding | Severity | File:Line |
| :--- | :--- | :--- |
| Split `src/cli.rs` | **P0** | `src/cli.rs:1` |
| Unified `doctor` errors | **P1** | `src/commands/doctor.rs:81` |
| Client-native `get_bytes` | **P1** | `src/commands/dubbing/mod.rs:89` |
| `dialogue` limit tests | **P1** | `src/commands/dialogue.rs:56` |
| Shared path heuristic | **P1** | `src/commands/align.rs:109` |
| Multipart error tests | **P2** | `tests/align_multipart.rs` |
