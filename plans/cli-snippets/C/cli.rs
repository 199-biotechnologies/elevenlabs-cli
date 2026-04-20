// === Worker C: dubbing ===
//
// 1) Add to `pub enum Commands { … }` in src/cli.rs:

    // ── Domain: dubbing ─────────────────────────────────────────────────────
    /// Dub media into new languages (and edit dubs Studio-style)
    Dubbing {
        #[command(subcommand)]
        action: DubbingAction,
    },

// 2) Append these enums at the bottom of src/cli.rs:

// ── Dubbing ────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum DubbingAction {
    /// Create a new dubbing job from a local file or URL
    #[command(visible_alias = "new")]
    Create(DubbingCreateArgs),

    /// List your dubbing jobs
    #[command(visible_alias = "ls")]
    List {
        /// Filter by status (dubbing | dubbed | failed | …)
        #[arg(long)]
        dubbing_status: Option<String>,

        /// Filter by creator: only_me | admin | workspace
        #[arg(long)]
        filter_by_creator: Option<String>,

        /// Page size (max 100)
        #[arg(long)]
        page_size: Option<u32>,
    },

    /// Get a dubbing job's full status
    #[command(visible_alias = "get")]
    Show {
        /// Dubbing job ID
        dubbing_id: String,
    },

    /// Delete a dubbing job
    #[command(visible_alias = "rm")]
    Delete {
        /// Dubbing job ID
        dubbing_id: String,

        /// Confirm deletion (required — deletion is irreversible).
        #[arg(long)]
        yes: bool,
    },

    /// Download the dubbed audio/video for a language
    GetAudio {
        /// Dubbing job ID
        dubbing_id: String,

        /// ISO language code (es, fr, de, …)
        language_code: String,

        /// Output file path (default: dub_<id>_<lang>.mp4)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Download the transcript for a dubbed language in the requested format
    GetTranscript {
        /// Dubbing job ID
        dubbing_id: String,

        /// ISO language code
        language_code: String,

        /// Transcript format
        #[arg(long, value_parser = ["srt", "webvtt", "json"])]
        format: String,

        /// Output file path (default: dub_<id>_<lang>.<ext>)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Editable-dub (Studio) operations — require `--dubbing-studio=true` at create
    Resource {
        #[command(subcommand)]
        action: DubbingResourceAction,
    },
}

#[derive(clap::Args, Debug, Clone)]
pub struct DubbingCreateArgs {
    /// Target language ISO code (required)
    #[arg(long)]
    pub target_lang: String,

    /// Source media file (mutually exclusive with --source-url)
    #[arg(long, conflicts_with = "source_url")]
    pub file: Option<String>,

    /// Publicly reachable URL to source media (mutually exclusive with --file)
    #[arg(long, conflicts_with = "file")]
    pub source_url: Option<String>,

    /// Source language ISO code (auto-detect when omitted)
    #[arg(long)]
    pub source_lang: Option<String>,

    /// Number of speakers in the source media (1-32)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=32))]
    pub num_speakers: Option<u32>,

    /// Embed the ElevenLabs watermark in the output
    #[arg(long)]
    pub watermark: Option<bool>,

    /// Start time of the clip to dub, in seconds
    #[arg(long)]
    pub start_time: Option<u32>,

    /// End time of the clip to dub, in seconds
    #[arg(long)]
    pub end_time: Option<u32>,

    /// Use the highest available video resolution for rendering
    #[arg(long)]
    pub highest_resolution: Option<bool>,

    /// Drop background audio (music/SFX) from the dubbed output
    #[arg(long)]
    pub drop_background_audio: Option<bool>,

    /// Run the profanity filter before dubbing
    #[arg(long)]
    pub use_profanity_filter: Option<bool>,

    /// Return a Studio-editable dub instead of a one-shot render
    #[arg(long)]
    pub dubbing_studio: Option<bool>,

    /// Disable voice cloning — use the default voice per speaker instead
    #[arg(long)]
    pub disable_voice_cloning: Option<bool>,

    /// Dubbing mode: automatic | manual
    #[arg(long, value_parser = ["automatic", "manual"])]
    pub mode: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DubbingResourceAction {
    /// Re-run transcription on the source media
    Transcribe {
        /// Dubbing job ID
        dubbing_id: String,

        /// JSON file with request-body overrides
        #[arg(long, value_name = "PATH")]
        patch: Option<String>,
    },

    /// Re-run translation from the transcript
    Translate {
        dubbing_id: String,

        #[arg(long, value_name = "PATH")]
        patch: Option<String>,
    },

    /// Re-run the dub step
    Dub {
        dubbing_id: String,

        #[arg(long, value_name = "PATH")]
        patch: Option<String>,
    },

    /// Re-render the dubbed output for a target language
    Render {
        dubbing_id: String,

        /// Target language ISO code
        language_code: String,

        #[arg(long, value_name = "PATH")]
        patch: Option<String>,
    },

    /// Migrate legacy segment metadata to the current schema
    MigrateSegments {
        dubbing_id: String,

        #[arg(long, value_name = "PATH")]
        patch: Option<String>,
    },
}
