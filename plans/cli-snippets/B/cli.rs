// ── Add to `Commands` enum in src/cli.rs ─────────────────────────────────────
//
// Insert after the `Sfx(SfxArgs)` variant (keeps speech-domain grouping).

/// Text-to-Dialogue: multi-speaker synthesis with `eleven_v3` flagship.
#[command(visible_alias = "dlg")]
Dialogue(DialogueArgs),

/// Forced alignment: align a known transcript to an audio recording.
Align(AlignArgs),

// ── Add to the bottom of src/cli.rs (after the existing `*Args`/`*Action`) ───

// ── Dialogue ───────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
pub struct DialogueArgs {
    /// Dialogue inputs as positional triples `label:voice_id:text`, e.g.
    /// `"Alice:v_1234:Hello there"`. Alternatively, pass a single JSON file
    /// path and the CLI will detect it via the `.json` extension. Pass `-`
    /// to read JSON from stdin.
    #[arg(value_name = "LINE")]
    pub positional: Vec<String>,

    /// Path to a JSON file containing an array of `{text, voice_id}`
    /// entries. Mutually exclusive with positional triples. Use `-` for stdin.
    #[arg(long, value_name = "PATH", conflicts_with = "positional")]
    pub input: Option<String>,

    /// Output file path. Defaults to ./dialogue_<timestamp>.<ext>.
    #[arg(short, long)]
    pub output: Option<String>,

    /// Model ID (default eleven_v3).
    #[arg(long)]
    pub model: Option<String>,

    /// Output format (e.g. mp3_44100_128, pcm_44100, ulaw_8000).
    #[arg(long)]
    pub format: Option<String>,

    /// Route to the streaming endpoint (/v1/text-to-dialogue/stream).
    /// Combine with --with-timestamps for NDJSON-style chunked timing data.
    #[arg(long)]
    pub stream: bool,

    /// Return per-character alignment alongside the audio. Saves audio to
    /// --output and alignment JSON to --save-timestamps.
    #[arg(long)]
    pub with_timestamps: bool,

    /// Path to save alignment JSON when --with-timestamps is set. Defaults
    /// to <audio>.timings.json.
    #[arg(long, value_name = "PATH")]
    pub save_timestamps: Option<String>,

    /// Write raw audio bytes to stdout instead of a file (implies --quiet).
    /// Only supported on the non-timestamp variants.
    #[arg(long)]
    pub stdout: bool,

    /// Sampling seed for reproducibility (0..=4_294_967_295).
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=4_294_967_295))]
    pub seed: Option<u32>,

    /// Stability 0.0-1.0.
    #[arg(long)]
    pub stability: Option<f32>,

    /// Similarity boost 0.0-1.0.
    #[arg(long)]
    pub similarity: Option<f32>,

    /// Style exaggeration 0.0-1.0.
    #[arg(long)]
    pub style: Option<f32>,

    /// Speaker boost (default on).
    #[arg(long)]
    pub speaker_boost: Option<bool>,

    /// ISO language code (mostly for v3; optional).
    #[arg(long)]
    pub language: Option<String>,

    /// Text normalization mode: auto | on | off.
    #[arg(long, value_parser = ["auto", "on", "off"])]
    pub apply_text_normalization: Option<String>,

    /// Latency optimization level (0=none, 4=max).
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=4))]
    pub optimize_streaming_latency: Option<u32>,

    /// Zero-retention mode (enterprise only).
    #[arg(long)]
    pub no_logging: bool,
}

// ── Align ──────────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
pub struct AlignArgs {
    /// Input audio file (the recording to align against).
    pub audio: String,

    /// Inline transcript text, OR a path to a transcript file (detected when
    /// the value is a short single-line path that exists on disk). For
    /// anything non-trivial prefer --transcript-file.
    #[arg(conflicts_with = "transcript_file")]
    pub transcript: Option<String>,

    /// Path to a transcript file. Use this for transcripts with newlines or
    /// paths containing colons.
    #[arg(long, value_name = "PATH")]
    pub transcript_file: Option<String>,

    /// Send the audio as a spooled file — required when the file is very
    /// large (>~50MB).
    #[arg(long)]
    pub enabled_spooled_file: bool,

    /// Save the full JSON response (characters + words) to a file.
    #[arg(short, long)]
    pub output: Option<String>,
}
