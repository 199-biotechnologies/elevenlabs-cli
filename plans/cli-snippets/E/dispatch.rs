// src/main.rs already routes Music to the music module — the existing
// arm is:
//
//     Commands::Music { action } => commands::music::dispatch(ctx, action).await,
//
// No change needed. The new MusicAction variants (Detailed, Stream, Upload,
// StemSeparation, VideoToMusic) are handled inside the reshaped
// `src/commands/music/mod.rs::dispatch`. If the existing match arm still
// exists in main.rs, leave it as-is.
