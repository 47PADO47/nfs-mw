//! Tests against the user's own install. They need game data, so they are
//! `#[ignore]`d; run them with
//!
//! ```sh
//! NFSMW_GAME_DIR="D:/Need For Speed Most Wanted Black Edition" cargo test --release -p nfsmw-data -- --ignored
//! ```
//!
//! Without `NFSMW_GAME_DIR` they pass without checking anything.

mod carparts;
mod cars;
mod compare;
mod exhaust;
mod ginsu;
mod handling;
mod manual;
mod minimap;
mod music;
mod physics;
mod sound;
mod textures;
mod world;
mod zones;

use game_install::GameDir;
use nfsmw_data::game::SPEC;

fn install() -> Option<GameDir> {
    let dir = std::env::var_os(SPEC.env_var)?;
    Some(GameDir::open(std::path::PathBuf::from(dir)).expect("NFSMW_GAME_DIR is set but cannot be indexed"))
}

fn unwrapped(dir: &GameDir, rel: &str) -> Vec<u8> {
    nfsmw_data::read_unwrapped(dir, rel).unwrap_or_else(|e| panic!("{rel}: {e:#}"))
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn install_validates() {
    let Some(dir) = install() else { return };
    let v = dir.validate(&SPEC);
    assert!(v.is_usable(), "missing: {:?}", v.missing);
}
