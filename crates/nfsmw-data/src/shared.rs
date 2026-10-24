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
const INGAME_B_BUN_FILE: &str = "GLOBAL/InGameB.bun";
/// The same pack as the game loads it (`GLOBAL\INGAMEB.LZC`, mod-world-content spec 1.1). Mod tools patch this
/// file, not the `.bun`, which stays as shipped.
const INGAME_B_LZC_FILE: &str = "GLOBAL/INGAMEB.LZC";
/// The English strings of the game, `LANGUAGES/English.bin`.
pub const LANGUAGE_FILE: &str = "LANGUAGES/English.bin";
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

/// The file of the in-game pack in `dir`: `GLOBAL/INGAMEB.LZC` as the game loads it, else `GLOBAL/InGameB.bun`.
/// Key a [`memo`] of what is read from the pack on this, so that it follows a change of file.
pub fn ingame_b_file(dir: &GameDir) -> &'static str {
    match dir.exists(INGAME_B_LZC_FILE) {
        true => INGAME_B_LZC_FILE,
        false => INGAME_B_BUN_FILE,
    }
}

/// The in-game pack (see [`ingame_b_file`]) with its compression wrapper removed, read once per session.
pub fn ingame_b(dir: &GameDir) -> Result<Arc<Vec<u8>>> {
    shared_unwrapped(dir, ingame_b_file(dir))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_in_game_pack_is_the_lzc_the_game_loads_when_there_is_one() {
        let root = std::env::temp_dir().join(format!("nfsmw-data-shared-ingame-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("GLOBAL")).unwrap();
        std::fs::write(root.join("GLOBAL/InGameB.bun"), b"shipped").unwrap();
        let dir = GameDir::open(&root).unwrap();
        assert_eq!(ingame_b_file(&dir), "GLOBAL/InGameB.bun");
        assert_eq!(ingame_b(&dir).unwrap().as_slice(), b"shipped");

        // A mod tool patched the .lzc (whatever the case of its name): that is the file the game reads.
        std::fs::write(root.join("GLOBAL/Ingameb.lzc"), b"patched").unwrap();
        let dir = GameDir::open(&root).unwrap();
        assert_eq!(ingame_b_file(&dir), "GLOBAL/INGAMEB.LZC");
        assert_eq!(ingame_b(&dir).unwrap().as_slice(), b"patched");
        std::fs::remove_dir_all(root).unwrap();
    }
}
