//! Session-shared reads of the install files that many readers need.
//!
//! Each accessor reads its file once per session and hands out the same `Arc` on every later call, so a
//! reader does not read, decompress or parse the file again. The bytes come from [`shared_unwrapped`] and the
//! parsed database from [`memo`]; both invalidate when the file changes (see `files.rs`).

use std::sync::Arc;

use anyhow::{Context, Result};
use blackbox_attrib::Database;
use game_install::GameDir;

use crate::{memo, shared_unwrapped};

/// The class definitions and records of `GLOBAL/ATTRIBUTES.BIN`. Read as stored, without a compression wrapper.
pub const ATTRIBUTES_FILE: &str = "GLOBAL/ATTRIBUTES.BIN";
/// The car tables and shared records of `GLOBAL/GLOBALB.LZC`.
pub const GLOBALB_FILE: &str = "GLOBAL/GLOBALB.LZC";
/// The in-game pack `GLOBAL/InGameB.bun`: the collision sounds, the exhaust particles and the in-game screens.
pub const INGAME_B_FILE: &str = "GLOBAL/InGameB.bun";
/// The English strings of the game, `LANGUAGES/English.bin`. The other languages are in [`crate::languages`].
pub const LANGUAGE_FILE: &str = crate::languages::ENGLISH_FILE;
/// The mix map of the car sound, `SOUND/MIXMAPS/MAPOUTPUT.mxb`.
pub const MAP_OUTPUT_FILE: &str = "SOUND/MIXMAPS/MAPOUTPUT.mxb";
/// The metadata bundle of the L2RA track, `TRACKS/L2RA.BUN`.
pub const L2RA_FILE: &str = "TRACKS/L2RA.BUN";

/// The attribute database of `GLOBAL/ATTRIBUTES.BIN`, parsed once per session.
///
/// Callers that add to the database must clone it (`Database::clone`) and change the clone; this copy stays as it is.
pub fn attributes(dir: &GameDir) -> Result<Arc<Database>> {
    let hit = memo(dir, &[ATTRIBUTES_FILE], || {
        let bytes = dir.read(ATTRIBUTES_FILE).with_context(|| format!("reading {ATTRIBUTES_FILE}"))?;
        let db = Database::open(&bytes).with_context(|| format!("parsing {ATTRIBUTES_FILE}"))?;
        Ok(Attributes(Arc::new(db)))
    })?;
    Ok(Arc::clone(&hit.0))
}

/// `GLOBAL/GLOBALB.LZC` with its compression wrapper removed, read once per session.
pub fn globalb(dir: &GameDir) -> Result<Arc<Vec<u8>>> {
    shared_unwrapped(dir, GLOBALB_FILE)
}

/// `GLOBAL/InGameB.bun` with its compression wrapper removed, read once per session.
pub fn ingame_b(dir: &GameDir) -> Result<Arc<Vec<u8>>> {
    shared_unwrapped(dir, INGAME_B_FILE)
}

/// `LANGUAGES/English.bin` with its compression wrapper removed, read once per session.
pub fn language(dir: &GameDir) -> Result<Arc<Vec<u8>>> {
    shared_unwrapped(dir, LANGUAGE_FILE)
}

/// `SOUND/MIXMAPS/MAPOUTPUT.mxb` with its compression wrapper removed, read once per session.
pub fn map_output(dir: &GameDir) -> Result<Arc<Vec<u8>>> {
    shared_unwrapped(dir, MAP_OUTPUT_FILE)
}

/// `TRACKS/L2RA.BUN` with its compression wrapper removed, read once per session.
pub fn l2ra(dir: &GameDir) -> Result<Arc<Vec<u8>>> {
    shared_unwrapped(dir, L2RA_FILE)
}

/// The memo entry of [`attributes`]. Its own type keeps it apart from any other memo of a `Database`.
struct Attributes(Arc<Database>);
