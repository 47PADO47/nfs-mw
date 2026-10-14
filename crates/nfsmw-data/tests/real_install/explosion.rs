//! The petrol station's explosion data (docs/specs/petrol-station-explosion.md) read from the install.

use blackbox_attrib::Database;
use nfsmw_data::sound::{collision_stitches, heaviest_impact};
use nfsmw_data::world::explosion_fx::{is_petrol_station_sequence, petrol_station_emitters};

use crate::{install, unwrapped};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_petrol_station_effect_has_its_four_emitters_with_textures() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let emitters = petrol_station_emitters(&db);
    assert_eq!(emitters.len(), 4, "fxex_gasstation has four emitters");
    assert!(emitters.iter().all(|e| e.texture != 0));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_gas_pump_sequence_is_a_petrol_station_blast_and_the_blast_sound_has_a_stitch() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    assert!(is_petrol_station_sequence(blackbox_attrib::vlt_hash("gaspumpexplosion_seq")));
    let sound = heaviest_impact(&db, "WALL").expect("a stitched wall impact");
    let (level, id) = (0..4).rev().find_map(|l| sound.levels[l].first().map(|&id| (l, id))).expect("a stitch level");
    let stitches = collision_stitches(&unwrapped(&dir, "GLOBAL/InGameB.bun"));
    let stitch = stitches.get(id as usize).expect("the stitch exists");
    assert!(!stitch.pieces.is_empty(), "level {level} stitch {id} has pieces");
}
