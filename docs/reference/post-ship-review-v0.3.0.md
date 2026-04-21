# elevenlabs-cli v0.3.0 — post-ship review

## 1. Bug scan (v0.2.2 → v0.3.0 diff)

- `src/commands/agents/llms.rs:17-41` — the human renderer is wired to the wrong response fields. The vendored spec's `LLMListResponseModel-Input` / `LLMInfoModel-Input` returns `llms[].llm`, not `llm_id`/`id`, and it does not expose `display_name` or `provider`. `elevenlabs agents llms` in human mode will therefore print blank IDs/names against the current API shape even though `--json` is fine. Minimal repro: render the spec example `{"llms":[{"llm":"gemini-2.5-flash", ...}]}` and the table comes out empty. Proposed fix: read `llm`, and use real fields such as reasoning/deprecation capabilities instead of non-existent display metadata.

- `src/commands/agents/knowledge.rs:298-326` — the human output for `agents knowledge search` is also wired to a stale response shape. The spec's `KnowledgeBaseContentSearchResult` returns `document { ... }` plus `search_snippet[]`; this code looks for `document_name`/`document_id` and `content`/`chunk`. On the current contract, successful searches render blank doc names and blank snippets. `--json` is fine; TTY output is not. Proposed fix: read `hit["document"]["name"]` or `["id"]`, then flatten `search_snippet`.

- `src/commands/phone/call.rs:69-74` — `--dynamic-variables` does not actually merge into `--client-data.dynamic_variables`; it replaces the entire object. Repro: `--client-data '{"dynamic_variables":{"a":1,"b":2}}' --dynamic-variables '{"b":9,"c":3}'` yields `{"dynamic_variables":{"b":9,"c":3}}`, dropping `a`. The help text promises a merge. Proposed fix: deep-merge keys into any existing `conversation_initiation_client_data.dynamic_variables` object.

- `src/commands/phone/call.rs:84-88` — `--record` is sent for both Twilio and SIP-trunk calls, but the vendored spec only defines `call_recording_enabled` on `Body_Handle_an_outbound_call_via_Twilio_v1_convai_twilio_outbound_call_post`, not the SIP-trunk request body. The shipped docs also say "Twilio / SIP trunk" in `src/cli.rs:1459-1461`, `src/help.rs:412-414`, `src/commands/agent_info.rs:229`, and `CHANGELOG.md:34`. Proposed fix: gate `call_recording_enabled` to Twilio only, or prove the SIP endpoint accepts the extra field and document that with a live trace.

- `src/cli.rs:1465-1469` — `--ringing-timeout-secs` accepts `0`, but the vendored `TelephonyCallConfig.ringing_timeout_secs` minimum is `1` and maximum is `999`. Right now the CLI punts that validation to the API and returns a 422. Proposed fix: add a clap range parser.

- `src/commands/conversations.rs:139-155` vs `src/client.rs:271-295` — the new `conversations audio` byte helper drifted from the shared error mapper. It redacts secrets, but it no longer extracts `detail.message` and no longer truncates large bodies. Repro: a 404/422 JSON error that would normally surface as a short message via `check_status()` now comes back as a raw JSON blob; a proxy HTML error page can be much larger. Proposed fix: move GET-bytes support into `ElevenLabsClient` or reuse the existing `check_status`/`extract_api_message` path.

## 2. Default + doc freshness

- `--llm` default is stale. Current value: `gemini-3.1-flash-lite-preview` in `src/cli.rs:1178-1181`, `src/help.rs:251-255`, `src/commands/agent_info.rs:155-170`, `src/commands/agents/agent_config.rs:50-54`, and `README.md:274`. The vendored OpenAPI `LLM` schema now defaults to `gemini-2.5-flash`, and ElevenLabs' Apr 7, 2026 blog post says Gemini 2.5 Flash is the new recommended default language model for Conversational AI. It should be `gemini-2.5-flash`.

- Two help examples are now stale and likely invalid: `src/help.rs:299` and `src/help.rs:365` still use `gemini-3.1-flash-preview`, which is not present in the current vendored `LLM` enum. They should be replaced with a current allowlisted model, most likely `gemini-2.5-flash`.

- `--model-id` default `eleven_flash_v2_5` looks current and should stay. The public models docs still recommend Flash v2.5 for Agents Platform and show no newer Flash model. Note that the OpenAPI `TTSConversationalConfig-Input.model_id` default is still `eleven_flash_v2`, so the CLI is more current than the spec here; I would keep the CLI value.

- Generic TTS default `eleven_multilingual_v2` in `src/config.rs:214-220` still matches the official models guide for quality/content-creation use cases. No change needed.

- STT default `scribe_v2` in `src/cli.rs:301-302` is still current in the official models docs. No change needed.

- `Creator+` tier references are stale. Current copy appears in `src/cli.rs:1196-1200`, `src/help.rs:262-266`, `src/help.rs:352`, and `src/commands/agent_info.rs:170`. Current public pricing/docs use `Creator`, `Pro`, `Scale`, and `Business`, and the Expressive Mode docs say Eleven v3 Conversational is priced the same as other agent TTS models and expressive mode is enabled by default with that model. The `Creator+` wording should be removed or replaced.

- The agent max-duration default is drifting from the spec. Current CLI/default docs say `300` seconds in `src/cli.rs:1204-1208`, `src/help.rs:251-255`, `src/commands/agent_info.rs:155-160`, `src/commands/agents/agent_config.rs:53-54`, and the create scaffold writes the passed value into `src/commands/agents/create.rs:122`. The vendored `ConversationConfig.max_duration_seconds` default is now `600`. If `300` is not a deliberate product decision, it should be bumped to `600`; if it is deliberate, it should be labelled as an opinionated CLI override instead of sounding like the platform default.

- The allowlist copy around `eleven_turbo_v2` / `eleven_turbo_v2_5` is stale in tone. The backend still appears to accept them, so they should remain valid inputs, but the official models docs now mark both as deprecated in favor of the Flash equivalents. The neutral allowlist text in `src/help.rs:256-261`, `src/help.rs:320-324`, `src/commands/agent_info.rs:169-170`, and `src/commands/agents/agent_config.rs:26-37` should explicitly mark the Turbo IDs as deprecated.

- README install docs are stale. `README.md:58-60` still says `brew tap 199-biotechnologies/tap`; repo policy now points users to `paperfoot/tap` and deprecates the old tap. That install block should be updated.

## 3. Wiring / completeness

- `agents llms` is wired end-to-end. CLI variant exists in `src/cli.rs:1142-1145`, dispatch/module wiring exists in `src/commands/agents/mod.rs:8,29`, the command module exists at `src/commands/agents/llms.rs`, and `agent-info` lists it at `src/commands/agent_info.rs:147`. Direct `--help` copy exists via the doc comment. Missing piece: no user-facing README mention.

- `agents signed-url` is wired end-to-end. CLI variant exists in `src/cli.rs:1147-1154`, dispatch/module wiring exists in `src/commands/agents/mod.rs:10,30`, the command module exists at `src/commands/agents/signed_url.rs`, and `agent-info` lists it at `src/commands/agent_info.rs:148`. Direct `--help` copy exists. Missing piece: no README mention.

- `agents knowledge {list,search,refresh}` is wired end-to-end. CLI variants exist in `src/cli.rs:1287-1328`, dispatch exists in `src/commands/agents/mod.rs:79,83-101`, the module exists at `src/commands/agents/knowledge.rs`, and `agent-info` lists all three at `src/commands/agent_info.rs:149-151`. Direct `--help` copy exists via doc comments. Missing piece: no README mention.

- `conversations audio` is wired end-to-end. CLI variant exists in `src/cli.rs:1409-1416`, dispatch exists in `src/commands/conversations.rs:21-24`, the command implementation exists in that same module, and `agent-info` lists it at `src/commands/agent_info.rs:220`. Direct `--help` copy exists. Missing piece: no README mention.

- `phone call --client-data`, `--record`, and `--ringing-timeout-secs` are wired end-to-end. CLI flags exist in `src/cli.rs:1448-1469`, dispatch plumbing exists in `src/commands/phone/mod.rs:38-58`, implementation exists in `src/commands/phone/call.rs`, `agent-info` lists them at `src/commands/agent_info.rs:222-230`, and long help exists in `src/help.rs:406-448`. Missing piece: README still documents only `--dynamic-variables` in `README.md:296-300`, and the `--record` copy is inaccurate for SIP-trunk calls.

- I did not find a missing `main.rs` dispatch arm, a missing `pub mod`, or a dead CLI variant for the newly shipped surfaces. The completeness gap is documentation, not routing.

## 4. Stale content sweep

- `docs/reference/spec-audit-v0.2.2.md` is now a historical pre-ship audit but is named and written like live guidance. Multiple "current CLI" statements are false after v0.3.0: the missing-command list at `docs/reference/spec-audit-v0.2.2.md:7-29`, the `phone call` omissions at `:60-63`, the agent path drift at `:139-145`, and the dubbing transcript drift at `:231-237`. This file should either be renamed to something like `spec-audit-pre-v0.3.0.md` or get a top-of-file banner saying it is historical and partly resolved.

- The same audit file also has now-wrong specifics: `docs/reference/spec-audit-v0.2.2.md:9` suggests `agents signed-url <agent_id> --client-data @client-data.json`, but that flag never shipped on `agents signed-url`; `:60` says `call_recording_enabled` exists on both Twilio and SIP outbound bodies, which is not true in the current vendored spec.

- `docs/reference/README.md:30-31` references `docs/reference/audit-prompt.md`, but that file is not in the repo.

- `README.md:48` still says the CLI is "Grounded against the official elevenlabs-js v2.43 SDK". v0.3.0 instead vendors `docs/reference/openapi.elevenlabs.json` and the release was explicitly driven by that snapshot plus audit. That grounding statement should point at the vendored OpenAPI snapshot or be removed.

- `src/help.rs:266`, `src/help.rs:352`, `src/cli.rs:1200`, and `src/commands/agent_info.rs:170` use "Creator+" / "at the time of writing" language that will keep aging badly. Those should be made versionless now.

## 5. Top 5 concrete fixes to ship in v0.3.1

- Fix `agents llms` response parsing. Files: `src/commands/agents/llms.rs`. Shape: switch human rendering to `llms[].llm`, then show fields that actually exist now (`available_reasoning_efforts`, deprecation info, context/token limits).

- Fix `agents knowledge search` response parsing. Files: `src/commands/agents/knowledge.rs`. Shape: read `document.name` / `document.id`, flatten `search_snippet`, optionally print `score`.

- Deep-merge `--dynamic-variables` into `--client-data.dynamic_variables`. Files: `src/commands/phone/call.rs`. Shape: if the client-data object already contains `dynamic_variables`, merge keys into that map instead of replacing it wholesale.

- Scope `--record` correctly and validate `--ringing-timeout-secs` client-side. Files: `src/commands/phone/call.rs`, `src/cli.rs`, `src/help.rs`, `src/commands/agent_info.rs`, optionally `CHANGELOG.md`. Shape: only send `call_recording_enabled` for Twilio, error or ignore on SIP trunk, and clamp timeout to `1..=999`.

- Refresh defaults/docs in one sweep. Files: `src/cli.rs`, `src/help.rs`, `src/commands/agent_info.rs`, `src/commands/agents/agent_config.rs`, `README.md`, `docs/reference/spec-audit-v0.2.2.md`, `docs/reference/README.md`. Shape: move the default LLM to `gemini-2.5-flash`, replace stale Gemini 3.1 example IDs, remove `Creator+`, update the Homebrew tap path, and clearly mark the old spec audit as historical.
