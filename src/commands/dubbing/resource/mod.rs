//! Editable-dub (Studio-mode) resource operations. Every op is a POST to
//! `/v1/dubbing/resource/{id}/<action>` with an optional JSON body supplied
//! via `--patch <PATH>`.
//!
//! Studio mode is opt-in at create time:
//!     elevenlabs dubbing create --file src.mp4 --target-lang es --dubbing-studio=true
//! Once the dub is in `dubbing_status=dubbed` you can iterate its transcript,
//! translation, dub, or per-language render via these endpoints.

pub mod dub;
pub mod migrate;
pub mod render;
pub mod transcribe;
pub mod translate;
