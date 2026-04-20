//! src/cli.rs — replacement struct definitions for the four P0/P1 music
//! endpoints whose HTTP contracts were rewritten in Fixer β.
//!
//! The lead must splice each struct below in place of the existing
//! definition (around lines 1055-1113 of src/cli.rs in the pre-fix
//! tree). No other section of cli.rs needs changes — the enum variants
//! (`UploadArgs`, `StemSeparationArgs`, `VideoToMusicArgs`) keep their
//! names and positions.
//!
//! Flag renames vs. the pre-fix CLI (MUST appear in the CHANGELOG):
//!
//!   music upload
//!     - DROPPED:  --name <NAME>                    (no server field)
//!     - DROPPED:  --composition-plan <PATH>        (no server field)
//!     - NEW:      --extract-composition-plan      (bool; bumps latency,
//!                 returns the plan inline in the JSON response)
//!
//!   music stem-separation
//!     - RENAMED:  positional SOURCE -> FILE        (file path only —
//!                 the song_id branch was removed; the API only accepts
//!                 a file upload per the SDK)
//!     - DROPPED:  --stems <STEM> (repeatable)      (not an API concept;
//!                 the server decides which stems to return)
//!     - NEW:      --output-format <codec_sr_br>    (query param)
//!     - NEW:      --stem-variation-id <ID>         (form field)
//!     - NEW:      --sign-with-c2pa                 (bool; form field)
//!     (--output-dir unchanged)
//!
//!   music video-to-music
//!     - DROPPED:  --model <ID>                     (no server field)
//!     - NEW:      --sign-with-c2pa                 (bool; form field)
//!     (positional FILE, --description, --tag repeatable, --format,
//!      --output all unchanged)
//!
//! `DetailedArgs` does NOT need a clap change — the flag surface is
//! identical; only the underlying HTTP contract changed
//! (multipart/mixed response).

// ── music upload ───────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
pub struct UploadArgs {
    /// Local audio file to upload
    pub file: String,

    /// Generate and return the composition plan for the uploaded song.
    /// Increases latency, but the returned plan can be piped straight
    /// into `music compose --composition-plan <file>`.
    #[arg(long = "extract-composition-plan")]
    pub extract_composition_plan: bool,
}

// ── music stem-separation ──────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
pub struct StemSeparationArgs {
    /// Local audio file to split into stems.
    ///
    /// The API only accepts a file upload — the pre-v0.2 `song_id`
    /// branch was not supported by the server and has been removed.
    #[arg(value_name = "FILE")]
    pub file: String,

    /// Directory to write the stem files into. Defaults to
    /// `./stems_<timestamp>`.
    #[arg(long, value_name = "DIR")]
    pub output_dir: Option<String>,

    /// Output audio format (codec_samplerate_bitrate, e.g. mp3_44100_128).
    /// Applies to every stem in the returned archive.
    #[arg(long = "output-format", value_name = "FORMAT")]
    pub output_format: Option<String>,

    /// Server-side stem variation id (opaque; see the ElevenLabs docs
    /// for accepted values).
    #[arg(long = "stem-variation-id", value_name = "ID")]
    pub stem_variation_id: Option<String>,

    /// Sign each generated mp3 with C2PA.
    #[arg(long = "sign-with-c2pa")]
    pub sign_with_c2pa: bool,
}

// ── music video-to-music ───────────────────────────────────────────────────

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

    /// Output audio format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,

    /// Output audio file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Sign the output with C2PA. Only relevant for mp3 outputs.
    #[arg(long = "sign-with-c2pa")]
    pub sign_with_c2pa: bool,
}
