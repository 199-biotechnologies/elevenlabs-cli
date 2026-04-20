// ─── cli.rs additions for Worker D (pronunciation dictionaries) ───────────
//
// Append to `src/cli.rs`.
//
// 1. Add this variant inside `enum Commands` (e.g. just after History):

/// Manage pronunciation dictionaries (IPA / alias lexicons)
Dict {
    #[command(subcommand)]
    action: DictAction,
},

// 2. Add the subcommand enum at the bottom of `src/cli.rs`:

// ── Dict (pronunciation dictionaries) ──────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum DictAction {
    /// List pronunciation dictionaries
    #[command(visible_alias = "ls")]
    List {
        /// Pagination cursor from a previous page
        #[arg(long)]
        cursor: Option<String>,

        /// Page size (max 100)
        #[arg(long)]
        page_size: Option<u32>,

        /// Filter by substring match on name
        #[arg(long)]
        search: Option<String>,
    },

    /// Upload a PLS/lexicon file as a new pronunciation dictionary
    AddFile {
        /// Dictionary name (shown in the library)
        name: String,

        /// Path to a PLS XML / lexicon file
        file: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Workspace access mode: admin | editor | viewer
        #[arg(long)]
        workspace_access: Option<String>,
    },

    /// Create a new dictionary from `--rule` / `--alias-rule` flags (no file)
    AddRules {
        /// Dictionary name
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Workspace access mode: admin | editor | viewer
        #[arg(long)]
        workspace_access: Option<String>,

        /// Phoneme rule WORD:PHONEME (IPA). Repeatable. The first `:` splits
        /// word from phoneme — IPA tone/length colons after that survive.
        #[arg(long = "rule", value_name = "WORD:PHONEME")]
        rule: Vec<String>,

        /// Alias rule WORD:ALIAS (spoken as alias). Repeatable.
        #[arg(long = "alias-rule", value_name = "WORD:ALIAS")]
        alias_rule: Vec<String>,
    },

    /// Show a dictionary's metadata
    #[command(visible_alias = "get")]
    Show {
        /// Dictionary ID
        id: String,
    },

    /// Update dictionary metadata or archive it
    Update {
        /// Dictionary ID
        id: String,

        /// New name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// Archive the dictionary (hides it from default listings;
        /// reversible server-side, no --yes required)
        #[arg(long)]
        archive: bool,
    },

    /// Download the rendered PLS XML for a dictionary version
    Download {
        /// Dictionary ID
        id: String,

        /// Version ID. Omit to use the latest version.
        #[arg(long)]
        version: Option<String>,

        /// Output file path (defaults to pronunciation_dictionary_<id>_<version>.pls)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Replace every rule in the dictionary (creates a new version)
    SetRules {
        /// Dictionary ID
        id: String,

        /// Phoneme rule WORD:PHONEME (IPA). Repeatable.
        #[arg(long = "rule", value_name = "WORD:PHONEME")]
        rule: Vec<String>,

        /// Alias rule WORD:ALIAS. Repeatable.
        #[arg(long = "alias-rule", value_name = "WORD:ALIAS")]
        alias_rule: Vec<String>,

        /// Case-sensitive matching when applying rules (true|false).
        /// Omit to keep the server default.
        #[arg(long)]
        case_sensitive: Option<bool>,

        /// Only match on whole-word boundaries (true|false).
        /// Omit to keep the server default.
        #[arg(long)]
        word_boundaries: Option<bool>,
    },

    /// Append new rules to an existing dictionary (creates a new version)
    AddRulesTo {
        /// Dictionary ID
        id: String,

        /// Phoneme rule WORD:PHONEME. Repeatable.
        #[arg(long = "rule", value_name = "WORD:PHONEME")]
        rule: Vec<String>,

        /// Alias rule WORD:ALIAS. Repeatable.
        #[arg(long = "alias-rule", value_name = "WORD:ALIAS")]
        alias_rule: Vec<String>,
    },

    /// Remove rules by their `string_to_replace` value (repeatable --word)
    RemoveRules {
        /// Dictionary ID
        id: String,

        /// The WORD whose rule should be dropped. Repeatable.
        #[arg(long, value_name = "WORD")]
        word: Vec<String>,
    },
}
