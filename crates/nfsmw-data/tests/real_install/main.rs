//! Tests against the user's own install. They need game data, so they are
//! `#[ignore]`d; run them with
//!
//! ```sh
//! NFSMW_GAME_DIR="<your-install>" cargo test --release -p nfsmw-data -- --ignored
//! ```
//!
//! Without `NFSMW_GAME_DIR` they pass without checking anything.

mod anim_banks;
mod anim_clips;
mod anim_codecs;
mod anim_codecs_f;
mod anim_objects;
mod anim_skeletons;
mod career;
mod carparts;
mod cars;
mod compare;
mod exhaust;
mod explosion;
mod frontend_camera;
mod gameplay;
mod geometry;
mod ginsu;
mod handling;
mod heli_sheet;
mod load_timing;
mod manual;
mod minimap;
mod music;
mod nis_audio;
mod nis_bundles;
mod nis_cameras;
mod nis_scenes;
mod objects;
mod physics;
mod pursuit;
mod race_barrier_routes;
mod race_barriers;
mod race_story;
mod reactions;
mod sequences;
mod skin;
mod sound;
mod spawners;
mod speech;
mod story_extras;
mod subtitles;
mod textures;
mod tire_profile;
mod traffic;
mod trailer_logs;
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
