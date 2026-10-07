//! The streamed city. `TRACKS/<track>.BUN` holds the streaming index;
//! `TRACKS/STREAM<track>.BUN` holds the sections (textures, solids, scenery).
//! See `docs/formats/maps.md`.
//!
//! Sections come in two kinds: map tiles (with a position) and shared sets
//! (V/X/Y/Z: shared models and textures). Every scenery instance resolves to a
//! solid in its own tile or in a shared set, never in another tile, so shared
//! sets stay loaded and tiles stream independently.

mod globals;
mod index;
mod section;
mod streamer;

pub use globals::{GLOBAL_TEXTURE_FILES, GlobalTextures, load_global_textures};
pub use index::WorldIndex;
pub use section::{SectionData, load_section, parse_section};
pub use streamer::{Streamer, StreamerEvent};

/// The only track in NFS: Most Wanted: the city of Rockport.
pub const DEFAULT_TRACK: &str = "L2RA";
