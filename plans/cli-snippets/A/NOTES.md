# Worker A — agents module overhaul

## Built (checklist)

### Deliverable 1 — P0: `agents add-knowledge` must attach the doc
- [x] POST `/v1/convai/knowledge-base/{url|file|text}` creates the doc
- [x] GET `/v1/convai/agents/{agent_id}` fetches current config
- [x] PATCH `/v1/convai/agents/{agent_id}` appends `{id, type, name, usage_mode: "auto"}`
  to `conversation_config.agent.prompt.knowledge_base` (preserves existing entries)
- [x] If PATCH fails after doc creation, the error includes the doc id and a
  `elevenlabs agents update` retry hint so no KB doc is orphaned silently
- [x] Integration test: `tests/agents_attach_kb.rs`
  - `add_knowledge_creates_doc_and_patches_agent` — verifies KB POST + agent
    PATCH are both called, PATCH body appends (not overwrites) KB entries,
    and sets `id`/`type`/`name`/`usage_mode: auto`
  - `add_knowledge_surfaces_doc_id_when_patch_fails` — PATCH 500 error
    message must contain the doc id

### Deliverable 2 — Bumped defaults (snippet only)
- [x] Snippet in `plans/cli-snippets/A/cli.rs` with:
  - `--llm` default `gemini-3.1-flash-lite-preview` (was `gemini-2.0-flash-001`)
  - `--model-id` default `eleven_flash_v2_5` (was `eleven_turbo_v2`)
  - Doc comments updated in lockstep
- Note for lead: REPLACE existing `AgentsAction::Create` defaults (do not add alongside)

### Deliverable 3 — New commands
- [x] `agents update <agent_id> --patch <json_file>` → PATCH `/v1/convai/agents/{id}`
- [x] `agents duplicate <agent_id> --name <new_name>` → POST `/v1/convai/agents/{id}/duplicate`
- [x] `agents tools list` → GET `/v1/convai/tools`
- [x] `agents tools show <tool_id>` → GET `/v1/convai/tools/{id}`
- [x] `agents tools create --config <json>` → POST `/v1/convai/tools`
- [x] `agents tools update <tool_id> --patch <json>` → PATCH `/v1/convai/tools/{id}`
- [x] `agents tools delete <tool_id> --yes` → DELETE `/v1/convai/tools/{id}` (irreversible; `--yes` required)
- [x] `agents tools deps <tool_id>` (alias: `dependents`) → GET `/v1/convai/tools/{id}/dependent-agents`

Tools create/update use the pass-through JSON-file pattern (surface too wide
to model every field as flags). Integration tests in `tests/agents_tools.rs`
cover list (endpoint wiring), create (JSON body pass-through), delete (both
`--yes` refusal and success), and deps (dependent-agents endpoint).

Integration tests in `tests/agents_update.rs` cover update pass-through, the
missing-patch-file error path (exit 3), and duplicate with name override.

### Deliverable 4 — Module split
`src/commands/agents.rs` deleted and replaced with a directory:
- `src/commands/agents/mod.rs` — dispatcher
- `src/commands/agents/list.rs`
- `src/commands/agents/show.rs`
- `src/commands/agents/create.rs`
- `src/commands/agents/update.rs` (new)
- `src/commands/agents/duplicate.rs` (new)
- `src/commands/agents/delete.rs`
- `src/commands/agents/knowledge.rs` (with P0 fix)
- `src/commands/agents/tools/mod.rs` (new; dispatcher)
- `src/commands/agents/tools/list.rs`
- `src/commands/agents/tools/show.rs`
- `src/commands/agents/tools/create.rs`
- `src/commands/agents/tools/update.rs`
- `src/commands/agents/tools/delete.rs`
- `src/commands/agents/tools/deps.rs`

All files focused, each well under 200 lines. Longest is knowledge.rs
(~170 lines) because it orchestrates 4 steps (validate → POST KB → GET
agent → PATCH agent) with explicit retry hints.

## Tested

- `tests/agents_attach_kb.rs` — P0 KB attach fix (2 cases)
- `tests/agents_update.rs` — update + duplicate (3 cases)
- `tests/agents_tools.rs` — tools family (5 cases)

All tests use `wiremock` and pattern-match the `tests/http_regression.rs`
approach: isolated `ELEVENLABS_CLI_CONFIG`, `ELEVENLABS_API_BASE_URL` set to
the mock, `ELEVENLABS_API_KEY` removed so config file wins.

Verified `cargo check` against an isolated copy of the repo with the
`cli.rs` snippet applied in-place: ZERO agents-related compile errors. The
only remaining `cargo check` errors in the main tree come from other
workers (music, phone) and will resolve once the lead integrates all
snippets.

## Client helper

`client.rs` already ships `patch_json<B, T>` with `#[allow(dead_code)]` —
no request to the lead needed. My modules call `client.patch_json(...)`
directly.

## Dependencies

No new Cargo deps needed. Everything reuses `reqwest`, `serde_json`,
`tokio::fs`, `wiremock` (dev-only).

## Integration checklist for the lead

1. **`src/cli.rs`** — REPLACE the existing `AgentsAction` enum wholesale
   with the version in `plans/cli-snippets/A/cli.rs`. Also APPEND the new
   `AgentsToolsAction` enum from the same file (placed alongside the
   other `#[derive(Subcommand, Debug, Clone)]` enums).
2. **`src/main.rs`** — no dispatch changes needed. `commands::agents::dispatch`
   still exists with the same signature; the new variants are handled inside.
3. **`src/commands/mod.rs`** — no changes needed. The existing `pub mod agents;`
   now resolves to the directory automatically.
4. **`src/commands/agent_info.rs`** — merge the entries from
   `plans/cli-snippets/A/agent-info.json` into the `commands` map (replace
   the current `agents *` entries wholesale; 6 new entries under
   `agents tools *`).
5. After integration, run `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test`.

## Open questions

None. All endpoints verified against elevenlabs.io docs:
- `POST /v1/convai/agents/{id}/duplicate` accepts `{ "name": string|null }`
- `GET /v1/convai/tools` lists tools
- `GET /v1/convai/tools/{id}/dependent-agents` lists dependent agents

Contributed by Claude Code (claude-opus-4-7[1m]).
