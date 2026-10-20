//! The minimap's map variant: the stage picks the variant, each variant reads its own tiles, and the binding switches
//! to them without a rebuild.

use blackbox_feng::{NodeKind, Runtime, fe_hash_upper};
use blackbox_minimap::{Calibration, Orientation, tile_name};
use nfsmw_career::regions::MapVariant;

use super::{MinimapBinding, usable_variant, variant_for_stage, variant_stem};
use crate::hud::minimap_tests::package;
use crate::hud::state::{HudState, MapPosition};

const MAP: Calibration = Calibration { origin: [0.0, 0.0], width: 8000.0 };

#[test]
fn the_career_stage_picks_the_variant_and_no_career_gives_the_full_map() {
    assert_eq!(variant_for_stage(None), MapVariant::Full);
    assert_eq!(variant_for_stage(Some(16)), MapVariant::Unlock1);
    assert_eq!(variant_for_stage(Some(13)), MapVariant::Unlock1);
    assert_eq!(variant_for_stage(Some(12)), MapVariant::Unlock2);
    assert_eq!(variant_for_stage(Some(9)), MapVariant::Unlock2);
    assert_eq!(variant_for_stage(Some(8)), MapVariant::Full);
    assert_eq!(variant_for_stage(Some(1)), MapVariant::Full);
}

#[test]
fn the_stem_is_the_map_file_name_without_its_extension() {
    assert_eq!(variant_stem(MapVariant::Full), "MINI_MAP");
    assert_eq!(variant_stem(MapVariant::Unlock1), "MINI_MAP_UNLOCK_1");
    assert_eq!(variant_stem(MapVariant::Unlock2), "MINI_MAP_UNLOCK_2");
    assert_eq!(tile_name(variant_stem(MapVariant::Unlock1), 9), "MINI_MAP_UNLOCK_1_CHOP9");
}

#[test]
fn an_unloaded_variant_falls_back_to_the_full_map() {
    assert_eq!(usable_variant(MapVariant::Unlock1, |v| v == MapVariant::Unlock1), MapVariant::Unlock1);
    assert_eq!(usable_variant(MapVariant::Unlock2, |v| v == MapVariant::Full), MapVariant::Full);
    assert_eq!(usable_variant(MapVariant::Full, |_| false), MapVariant::Full);
}

fn position_at(position: [f32; 2]) -> MapPosition {
    MapPosition { position, heading: [0.0, 1.0], orientation: Orientation::North }
}

fn state(position: [f32; 2]) -> (HudState, MapPosition) {
    let hud = HudState { minimap: Some(position_at(position)), ..HudState::default() };
    (hud, position_at(position))
}

fn piece_texture(rt: &Runtime, id: blackbox_feng::PackageId, name: &str) -> u32 {
    let node = rt.tree(id).nodes.into_iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap();
    let NodeKind::Image { texture, .. } = node.kind else { panic!("{name} is not an image") };
    texture
}

#[test]
fn switching_the_variant_moves_the_pieces_to_its_tiles() {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let mut binding = MinimapBinding::new(&rt, id, MAP, "MINI_MAP").expect("the objects are found");
    let (hud, at) = state([1250.0, 6750.0]);

    binding.apply(&mut rt, &hud, &at);
    assert_eq!(piece_texture(&rt, id, "TRACK_MAP1"), fe_hash_upper("MINI_MAP_CHOP0"));

    assert!(binding.set_stem(variant_stem(MapVariant::Unlock1)), "the stem changed");
    binding.apply(&mut rt, &hud, &at);
    assert_eq!(piece_texture(&rt, id, "TRACK_MAP1"), fe_hash_upper("MINI_MAP_UNLOCK_1_CHOP0"));
    assert_eq!(piece_texture(&rt, id, "TRACK_MAP4"), fe_hash_upper("MINI_MAP_UNLOCK_1_CHOP9"));

    assert!(!binding.set_stem("MINI_MAP_UNLOCK_1"), "the same stem changes nothing");

    assert!(binding.set_stem(variant_stem(MapVariant::Full)), "back to the full map");
    binding.apply(&mut rt, &hud, &at);
    assert_eq!(piece_texture(&rt, id, "TRACK_MAP1"), fe_hash_upper("MINI_MAP_CHOP0"));
}

#[test]
fn a_race_event_has_its_own_tiles_and_the_stage_map_comes_back() {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let mut binding = MinimapBinding::new(&rt, id, MAP, "MINI_MAP_UNLOCK_1").expect("the objects are found");
    let (hud, at) = state([1250.0, 6750.0]);

    assert!(binding.set_stem("MINI_MAP_10_2_1"));
    binding.apply(&mut rt, &hud, &at);
    assert_eq!(piece_texture(&rt, id, "TRACK_MAP1"), fe_hash_upper("MINI_MAP_10_2_1_CHOP0"));
    assert_eq!(piece_texture(&rt, id, "TRACK_MAP4"), fe_hash_upper("MINI_MAP_10_2_1_CHOP9"));

    assert!(binding.set_stem("mini_map_10_4_2_r"), "the stem is kept in upper case");
    binding.apply(&mut rt, &hud, &at);
    assert_eq!(piece_texture(&rt, id, "TRACK_MAP1"), fe_hash_upper("MINI_MAP_10_4_2_R_CHOP0"));
    assert!(!binding.set_stem("MINI_MAP_10_4_2_R"));

    assert!(binding.set_stem("MINI_MAP_UNLOCK_1"), "the race ended: the stage's map again");
    binding.apply(&mut rt, &hud, &at);
    assert_eq!(piece_texture(&rt, id, "TRACK_MAP1"), fe_hash_upper("MINI_MAP_UNLOCK_1_CHOP0"));
}
