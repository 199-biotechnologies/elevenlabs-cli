# cli.rs attach map — Worker I

Attach each `help::*_HELP` constant to the corresponding clap struct / enum
variant in `src/cli.rs` via `#[command(after_long_help = help::XXX_HELP)]`.

For struct-based args (e.g. `TtsArgs`), the attribute sits on the `#[derive(clap::Args)]`
struct itself. For enum variants (e.g. `AgentsAction::Create`), the attribute
sits on the specific variant.

| Constant | Attach to (in `src/cli.rs`) | Attribute |
|---|---|---|
| `help::TTS_HELP` | `TtsArgs` struct | `#[command(after_long_help = help::TTS_HELP)]` |
| `help::STT_HELP` | `SttArgs` struct | `#[command(after_long_help = help::STT_HELP)]` |
| `help::SFX_HELP` | `SfxArgs` struct | `#[command(after_long_help = help::SFX_HELP)]` |
| `help::VOICES_LIBRARY_HELP` | `VoicesAction::Library` variant | `#[command(after_long_help = help::VOICES_LIBRARY_HELP)]` |
| `help::VOICES_DESIGN_HELP` | `VoicesAction::Design` variant | `#[command(after_long_help = help::VOICES_DESIGN_HELP)]` |
| `help::DIALOGUE_HELP` | `DialogueArgs` struct | `#[command(after_long_help = help::DIALOGUE_HELP)]` |
| `help::ALIGN_HELP` | `AlignArgs` struct | `#[command(after_long_help = help::ALIGN_HELP)]` |
| `help::MUSIC_COMPOSE_HELP` | `MusicAction::Compose` variant (holds `ComposeArgs`) | `#[command(after_long_help = help::MUSIC_COMPOSE_HELP)]` |
| `help::AGENTS_CREATE_HELP` | `AgentsAction::Create` variant | `#[command(after_long_help = help::AGENTS_CREATE_HELP)]` |
| `help::AGENTS_ADD_KNOWLEDGE_HELP` | `AgentsAction::AddKnowledge` variant | `#[command(after_long_help = help::AGENTS_ADD_KNOWLEDGE_HELP)]` |
| `help::PHONE_CALL_HELP` | `PhoneAction::Call` variant | `#[command(after_long_help = help::PHONE_CALL_HELP)]` |
| `help::DUBBING_CREATE_HELP` | `DubbingAction::Create` variant (holds `DubbingCreateArgs`) | `#[command(after_long_help = help::DUBBING_CREATE_HELP)]` |
| `help::DICT_ADD_RULES_HELP` | `DictAction::AddRules` variant | `#[command(after_long_help = help::DICT_ADD_RULES_HELP)]` |
| `help::DOCTOR_HELP` | `Commands::Doctor` variant (or `DoctorArgs` if Worker H made it a struct) | `#[command(after_long_help = help::DOCTOR_HELP)]` |
| `help::CONFIG_INIT_HELP` | `ConfigAction::Init` variant | `#[command(after_long_help = help::CONFIG_INIT_HELP)]` |
| `help::UPDATE_HELP` | `Commands::Update` variant | `#[command(after_long_help = help::UPDATE_HELP)]` |
| `help::SKILL_INSTALL_HELP` | `SkillAction::Install` variant | `#[command(after_long_help = help::SKILL_INSTALL_HELP)]` |
| `help::HISTORY_LIST_HELP` | `HistoryAction::List` variant | `#[command(after_long_help = help::HISTORY_LIST_HELP)]` |
| `help::USER_SUBSCRIPTION_HELP` | `UserAction::Subscription` variant | `#[command(after_long_help = help::USER_SUBSCRIPTION_HELP)]` |

## Integration notes for the lead

1. Add `pub mod help;` near the top of `src/main.rs` (see `mod.txt`). If the
   binary uses `lib.rs`, put it there instead.
2. All constants are `pub const NAME: &str` — no runtime cost, compiled in
   as static data.
3. Each string uses ASCII-only characters (IPA examples in the docstrings
   contain real UTF-8 IPA characters, but those are in actual help strings,
   not code identifiers). Safe to embed as-is.
4. clap's `after_long_help` only shows up under `--help` (long form); it does
   NOT show up under `-h`. That's intentional — `-h` stays compact.
5. If any struct name shifts during Phase 2 integration (e.g. `ComposeArgs`
   → `MusicComposeArgs`), the attach attribute moves with the struct — just
   keep the constant → struct mapping consistent.
