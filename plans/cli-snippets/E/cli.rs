// Paste REPLACING the existing `MusicAction` enum in `src/cli.rs`.
// Adds ComposeArgs / DetailedArgs / StreamArgs / UploadArgs /
// StemSeparationArgs / VideoToMusicArgs so the `music::*` submodules can
// accept a single struct instead of the current tuple-shaped variants.

// ── Music ──────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum MusicAction {
    /// Compose music from a text prompt
    Compose(ComposeArgs),

    /// Create a composition plan (free, subject to rate limits)
    Plan {
        /// Text prompt
        prompt: String,

        /// Target length in milliseconds (3000-600000)
        #[arg(long, value_parser = clap::value_parser!(u32).range(3000..=600000))]
        length_ms: Option<u32>,

        /// Model ID (default music_v1)
        #[arg(long)]
        model: Option<String>,
    },

    /// Generate music with rich metadata (bpm, time_signature, sections).
    /// Audio and metadata are written to separate files.
    Detailed(DetailedArgs),

    /// Stream the generated audio to disk as bytes arrive
    Stream(StreamArgs),

    /// Upload an audio file so it can be referenced by song_id for inpainting
    Upload(UploadArgs),

    /// Split a track into stems (vocals/drums/bass/other)
    #[command(name = "stem-separation", visible_alias = "stems")]
    StemSeparation(StemSeparationArgs),

    /// Generate a score from video content (Apr 2026)
    #[command(name = "video-to-music", visible_alias = "v2m")]
    VideoToMusic(VideoToMusicArgs),
}

#[derive(clap::Args, Debug, Clone)]
pub struct ComposeArgs {
    /// Text prompt. Mutually exclusive with --composition-plan.
    #[arg(required_unless_present = "composition_plan")]
    pub prompt: Option<String>,

    /// Target length in milliseconds. Must be 3000-600000. Only used with --prompt.
    #[arg(long, value_parser = clap::value_parser!(u32).range(3000..=600000))]
    pub length_ms: Option<u32>,

    /// Output audio format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Path to a composition-plan JSON file (mutually exclusive with PROMPT).
    #[arg(long, value_name = "PATH", conflicts_with_all = ["prompt", "length_ms", "force_instrumental"])]
    pub composition_plan: Option<String>,

    /// Model ID (default music_v1)
    #[arg(long)]
    pub model: Option<String>,

    /// Force the output to be instrumental. Only valid with --prompt.
    #[arg(long)]
    pub force_instrumental: bool,

    /// Seed for reproducibility (cannot be combined with --prompt)
    #[arg(long)]
    pub seed: Option<u32>,

    /// Strictly enforce per-section durations from the composition plan.
    #[arg(long)]
    pub respect_sections_durations: bool,

    /// Store the generated song for inpainting (enterprise only)
    #[arg(long)]
    pub store_for_inpainting: bool,

    /// Sign the output mp3 with C2PA
    #[arg(long)]
    pub sign_with_c2pa: bool,
}

#[derive(clap::Args, Debug, Clone)]
pub struct DetailedArgs {
    /// Text prompt. Mutually exclusive with --composition-plan.
    #[arg(required_unless_present = "composition_plan")]
    pub prompt: Option<String>,

    /// Target length in milliseconds (3000-600000). Only used with --prompt.
    #[arg(long, value_parser = clap::value_parser!(u32).range(3000..=600000))]
    pub length_ms: Option<u32>,

    /// Output audio format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,

    /// Output audio file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Path to save the metadata JSON (defaults to <output>.metadata.json)
    #[arg(long, value_name = "PATH")]
    pub save_metadata: Option<String>,

    /// Path to a composition-plan JSON file
    #[arg(long, value_name = "PATH", conflicts_with_all = ["prompt", "length_ms", "force_instrumental"])]
    pub composition_plan: Option<String>,

    /// Model ID (default music_v1)
    #[arg(long)]
    pub model: Option<String>,

    /// Force instrumental output.
    #[arg(long)]
    pub force_instrumental: bool,

    /// Seed for reproducibility
    #[arg(long)]
    pub seed: Option<u32>,

    /// Respect per-section durations
    #[arg(long)]
    pub respect_sections_durations: bool,

    /// Store song for inpainting
    #[arg(long)]
    pub store_for_inpainting: bool,

    /// Sign output with C2PA
    #[arg(long)]
    pub sign_with_c2pa: bool,
}

#[derive(clap::Args, Debug, Clone)]
pub struct StreamArgs {
    /// Text prompt. Mutually exclusive with --composition-plan.
    #[arg(required_unless_present = "composition_plan")]
    pub prompt: Option<String>,

    /// Target length in milliseconds (3000-600000)
    #[arg(long, value_parser = clap::value_parser!(u32).range(3000..=600000))]
    pub length_ms: Option<u32>,

    /// Output audio format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Path to a composition-plan JSON file
    #[arg(long, value_name = "PATH", conflicts_with_all = ["prompt", "length_ms", "force_instrumental"])]
    pub composition_plan: Option<String>,

    /// Model ID (default music_v1)
    #[arg(long)]
    pub model: Option<String>,

    /// Force instrumental output.
    #[arg(long)]
    pub force_instrumental: bool,

    /// Seed for reproducibility
    #[arg(long)]
    pub seed: Option<u32>,

    /// Respect per-section durations
    #[arg(long)]
    pub respect_sections_durations: bool,

    /// Store song for inpainting
    #[arg(long)]
    pub store_for_inpainting: bool,

    /// Sign output with C2PA
    #[arg(long)]
    pub sign_with_c2pa: bool,
}

#[derive(clap::Args, Debug, Clone)]
pub struct UploadArgs {
    /// Local audio file to upload
    pub file: String,

    /// Friendly name for the song
    #[arg(long)]
    pub name: Option<String>,

    /// Path to a composition-plan JSON file to attach
    #[arg(long, value_name = "PATH")]
    pub composition_plan: Option<String>,
}

#[derive(clap::Args, Debug, Clone)]
pub struct StemSeparationArgs {
    /// Either a local audio file path or a song_id from `music upload`
    #[arg(value_name = "SONG_ID_OR_FILE")]
    pub source: String,

    /// Directory to write the stem files into. Defaults to ./stems_<timestamp>.
    #[arg(long, value_name = "DIR")]
    pub output_dir: Option<String>,

    /// Stems to extract. Repeatable. Default set: vocals, drums, bass, other.
    #[arg(long = "stems", value_name = "STEM", default_values_t = vec![
        "vocals".to_string(),
        "drums".to_string(),
        "bass".to_string(),
        "other".to_string(),
    ])]
    pub stems: Vec<String>,
}

#[derive(clap::Args, Debug, Clone)]
pub struct VideoToMusicArgs {
    /// Input video file
    pub file: String,

    /// Optional text description to steer the score
    #[arg(long)]
    pub description: Option<String>,

    /// Style / mood tags (repeatable)
    #[arg(long = "tag", value_name = "TAG")]
    pub tags: Vec<String>,

    /// Model ID override
    #[arg(long)]
    pub model: Option<String>,

    /// Output audio format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,

    /// Output audio file path
    #[arg(short, long)]
    pub output: Option<String>,
}
