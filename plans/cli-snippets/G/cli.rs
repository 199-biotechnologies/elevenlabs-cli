// Replace the entire `pub enum VoicesAction { ... }` in src/cli.rs with
// this version. Adds new subcommands (add-shared, similar, edit) and
// extends `list` with the full v2 query-param set (Apr 13, 2026 additions
// including voice_type=non-community and include_total_count).

#[derive(Subcommand, Debug, Clone)]
pub enum VoicesAction {
    /// List voices in your library (v2)
    #[command(visible_alias = "ls")]
    List {
        /// Filter by search term (matches name, description, labels, category)
        #[arg(long)]
        search: Option<String>,

        /// Sort field (name|created_at_unix)
        #[arg(long, default_value = "name")]
        sort: String,

        /// Sort direction (asc|desc)
        #[arg(long, default_value = "asc")]
        direction: String,

        /// Max results per page (1-100)
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Include legacy premade voices (/v1 compatibility)
        #[arg(long)]
        show_legacy: bool,

        /// Pagination cursor from a previous response
        #[arg(long, value_name = "TOKEN")]
        next_page_token: Option<String>,

        /// Voice type filter: personal|community|default|workspace|non-default|non-community|saved
        #[arg(long, value_parser = ["personal", "community", "default", "workspace", "non-default", "non-community", "saved"])]
        voice_type: Option<String>,

        /// Category filter: premade|cloned|generated|professional
        #[arg(long, value_parser = ["premade", "cloned", "generated", "professional"])]
        category: Option<String>,

        /// Fine-tuning state (professional voices only)
        #[arg(long, value_parser = ["draft", "not_verified", "not_started", "queued", "fine_tuning", "fine_tuned", "failed", "delayed"])]
        fine_tuning_state: Option<String>,

        /// Filter by collection ID
        #[arg(long)]
        collection_id: Option<String>,

        /// Include the total voice count in the response
        #[arg(long)]
        include_total_count: bool,

        /// Look up specific voice IDs (repeatable, max 100)
        #[arg(long = "voice-id", value_name = "ID")]
        voice_id: Vec<String>,
    },

    /// Get full details for a voice
    #[command(visible_alias = "get")]
    Show {
        /// Voice ID
        voice_id: String,
    },

    /// Search your voice library
    Search {
        /// Search term
        query: String,
    },

    /// Search the public (shared) voice library
    Library {
        /// Search term
        #[arg(long)]
        search: Option<String>,

        /// Page number (1-indexed)
        #[arg(long, default_value = "1")]
        page: u32,

        /// Page size (1-100)
        #[arg(long, default_value = "20")]
        page_size: u32,

        /// Category: professional | high_quality | famous
        #[arg(long)]
        category: Option<String>,

        /// Gender filter (male|female|neutral)
        #[arg(long)]
        gender: Option<String>,

        /// Age filter (young|middle_aged|old)
        #[arg(long)]
        age: Option<String>,

        /// Accent filter
        #[arg(long)]
        accent: Option<String>,

        /// Language filter (ISO code)
        #[arg(long)]
        language: Option<String>,

        /// Locale filter (e.g. en-US)
        #[arg(long)]
        locale: Option<String>,

        /// Use case filter (narration|audiobook|...)
        #[arg(long)]
        use_case: Option<String>,

        /// Filter to featured voices only
        #[arg(long)]
        featured: bool,

        /// Filter voices by minimum notice period in days
        #[arg(long)]
        min_notice_days: Option<u32>,

        /// Include voices with custom rates
        #[arg(long)]
        include_custom_rates: bool,

        /// Include live-moderated voices
        #[arg(long)]
        include_live_moderated: bool,

        /// Filter voices enabled for the reader app
        #[arg(long)]
        reader_app_enabled: bool,

        /// Filter by public owner ID
        #[arg(long)]
        owner_id: Option<String>,

        /// Sort criteria (e.g. cloned_by_count, usage_character_count_1y)
        #[arg(long)]
        sort: Option<String>,
    },

    /// Instant-clone a voice from sample audio files (IVC)
    Clone {
        /// Name for the cloned voice
        name: String,

        /// Audio sample files (mp3/wav/m4a)
        #[arg(required = true)]
        files: Vec<String>,

        /// Description for the voice
        #[arg(long)]
        description: Option<String>,
    },

    /// Generate voice previews from a text description (voice design)
    Design {
        /// Text description of the voice
        description: String,

        /// Optional text to read in the preview (auto-generated if omitted).
        /// Must be 100-1000 characters.
        #[arg(long)]
        text: Option<String>,

        /// Directory to save preview files
        #[arg(long)]
        output_dir: Option<String>,

        /// Voice design model: eleven_multilingual_ttv_v2 | eleven_ttv_v3
        #[arg(long, value_parser = ["eleven_multilingual_ttv_v2", "eleven_ttv_v3"])]
        model: Option<String>,

        /// Loudness -1.0 (quietest) to 1.0 (loudest); 0 ≈ -24 LUFS
        #[arg(long)]
        loudness: Option<f32>,

        /// Seed for reproducible generation
        #[arg(long)]
        seed: Option<u32>,

        /// Guidance scale — higher = stick closer to prompt (may sound robotic)
        #[arg(long)]
        guidance_scale: Option<f32>,

        /// Enhance the description with AI before generation
        #[arg(long)]
        enhance: bool,

        /// Return preview IDs only (stream audio via separate endpoint)
        #[arg(long)]
        stream_previews: bool,

        /// Higher quality = better voice but less variety
        #[arg(long)]
        quality: Option<f32>,
    },

    /// Save a previously-designed voice preview to your library
    SavePreview {
        /// Generated voice ID (from `voices design`)
        generated_voice_id: String,

        /// Voice name
        name: String,

        /// Voice description
        description: String,
    },

    /// Delete a voice
    #[command(visible_alias = "rm")]
    Delete {
        /// Voice ID to delete
        voice_id: String,

        /// Confirm deletion. Without this flag the command errors out instead
        /// of silently deleting — required because deletion is irreversible.
        #[arg(long)]
        yes: bool,
    },

    /// Add a shared voice (from the public library) to your collection
    AddShared {
        /// Public user ID that owns the shared voice
        public_user_id: String,

        /// Voice ID in the public library
        voice_id: String,

        /// Name to save the voice under in your library
        #[arg(long)]
        name: String,

        /// Mark the voice as bookmarked after adding
        #[arg(long)]
        bookmarked: Option<bool>,
    },

    /// Find shared voices similar to an audio sample
    Similar {
        /// Audio file to use as the reference sample
        audio_file: String,

        /// Similarity threshold 0.0-2.0 (lower = more similar)
        #[arg(long)]
        similarity_threshold: Option<f32>,

        /// Maximum voices to return (1-100)
        #[arg(long)]
        top_k: Option<u32>,

        /// Gender filter
        #[arg(long)]
        gender: Option<String>,

        /// Age filter
        #[arg(long)]
        age: Option<String>,

        /// Accent filter
        #[arg(long)]
        accent: Option<String>,

        /// Language filter
        #[arg(long)]
        language: Option<String>,

        /// Use case filter
        #[arg(long)]
        use_case: Option<String>,
    },

    /// Edit a voice — rename, re-describe, update labels, add/remove samples
    Edit {
        /// Voice ID to edit
        voice_id: String,

        /// New name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// Label pair (key=value). Repeatable.
        #[arg(long = "labels", value_name = "KEY=VALUE")]
        labels: Vec<String>,

        /// Additional sample file to upload. Repeatable.
        #[arg(long = "add-sample", value_name = "FILE")]
        add_sample: Vec<String>,

        /// Sample ID to remove. Repeatable.
        #[arg(long = "remove-sample", value_name = "SAMPLE_ID")]
        remove_sample: Vec<String>,

        /// Run added samples through the background-noise-removal model
        #[arg(long)]
        remove_background_noise: bool,
    },
}
