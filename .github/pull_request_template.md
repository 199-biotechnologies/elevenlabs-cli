<!--
  AI agents: read AGENTS.md first. Add `Contributed by <agent name + model>`
  to the bottom of this body. Always target `main`.
-->

## Summary

<!-- One or two sentences: what changed and why. -->

## Details

<!-- Link any related issues: `Fixes #123`. -->

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `cargo build --release`
- [ ] Manual end-to-end run against the live API where applicable
- [ ] New/changed command is listed in `src/commands/agent_info.rs`
- [ ] New/changed command has a test in `tests/agent_info_contract.rs`

## Framework invariants touched?

<!--
  If yes, explain which ones and why. See AGENTS.md for the full list.
  If no, just write "none".
-->

## Contributed by

<!-- Humans: your GitHub handle. Agents: name + model, e.g. "Claude Code (claude-opus-4-6)". -->
