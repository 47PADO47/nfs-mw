//! The signs and traffic lights conversion on authored 2012-style models: a sign on a pole, a mast arm with heads, a signal head. No game
//! data; the real models are read by `real_tests.rs`.

use std::collections::{HashMap, HashSet};

use blackbox_hash::bstring_hash;
use blackbox_modding::content::parse_replace_set;
use blackbox_solid::read_solids;
use blackbox_tpk::read_texture_packs;
use chameleon_gfx::{PixelFormat as SourceFormat, Texture, mip_size};

use super::build::{self, texture_name};
use super::table::{self, ARM_SWAPS, Box3, HEAD_SWAPS, SIGN_SWAPS, SIGNS};
use crate::trees::read::{Level, Part, Role, TreeModel};

const ATLAS: u64 = 0x0100_0000_0009_dbf8;
const PLATE: u64 = 0x0100_0000_0000_0042;

fn picture(width: u32, height: u32) -> Texture {
    let size = mip_size(SourceFormat::Bc1, width, height, 0).unwrap();
    Texture {
        width,
        height,
        depth: 1,
        array_size: 1,
        mip_levels: 1,
        dxgi_format: 71,
        format: SourceFormat::Bc1,
        data: vec![0x55; size],
    }
}

fn part(role: Role, texture: u64, positions: Vec<[f32; 3]>, triangles: Vec<[u32; 3]>) -> Part {
    let uvs = positions.iter().map(|p| [p[0], p[1]]).collect();
    Part { role, positions, uvs, triangles, texture }
}

/// A sign in the 2012 axes: a pole 2.3 m tall and a plate 0.6 wide near its top, facing +z.
fn sign() -> TreeModel {
    let plate = part(
        Role::Bark,
        PLATE,
        vec![[-0.3, 1.5, 0.03], [0.3, 1.5, 0.03], [0.3, 2.3, 0.03], [-0.3, 2.3, 0.03]],
        vec![[0, 1, 2], [0, 2, 3]],
    );
    let pole = part(
        Role::Bark,
        PLATE,
        vec![[-0.03, 0.0, 0.0], [0.03, 0.0, 0.0], [0.03, 1.5, 0.0], [-0.03, 1.5, 0.0]],
        vec![[0, 1, 2], [0, 2, 3]],
    );
    TreeModel { levels: vec![Level { parts: vec![plate, pole] }], textures: HashMap::from([(PLATE, picture(64, 64))]) }
}

/// A box of triangles with its faces out, from `low` to `high`.
fn cuboid(low: [f32; 3], high: [f32; 3]) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let mut points = Vec::new();
    for y in [low[1], high[1]] {
        for (x, z) in [(low[0], low[2]), (high[0], low[2]), (high[0], high[2]), (low[0], high[2])] {
            points.push([x, y, z]);
        }
    }
    let mut triangles = Vec::new();
    for side in 0..4u32 {
        let next = (side + 1) % 4;
        triangles.push([side, next, 4 + next]);
        triangles.push([side, 4 + next, 4 + side]);
    }
    (points, triangles)
}

/// Appends `parts` to one mesh (as the 2012 arm is one mesh).
fn merged(role: Role, texture: u64, boxes: &[([f32; 3], [f32; 3])]) -> Part {
    let (mut positions, mut triangles) = (Vec::new(), Vec::new());
    for (low, high) in boxes {
        let (p, t) = cuboid(*low, *high);
        let at = positions.len() as u32;
        positions.extend(p);
        triangles.extend(t.into_iter().map(|t| t.map(|i| i + at)));
    }
    part(role, texture, positions, triangles)
}

/// A mast arm in the 2012 axes: a pole 6 m tall at the origin, an arm 5.9 m long towards -x at the top, a head hanging from it.
fn arm() -> TreeModel {
    let mesh = merged(
        Role::Bark,
        ATLAS,
        &[
            ([-0.1, 0.0, -0.1], [0.1, 5.5, 0.1]),   // the pole
            ([-5.9, 5.4, -0.04], [0.0, 5.6, 0.04]), // the arm
            ([-2.4, 4.6, -0.1], [-2.1, 5.5, 0.2]),  // a signal head hanging from the arm
        ],
    );
    TreeModel { levels: vec![Level { parts: vec![mesh] }; 3], textures: HashMap::from([(ATLAS, picture(64, 64))]) }
}

/// A signal head in the 2012 axes (as `read` returns it): its own mesh, a housing and a lens visor in front of it, facing +z.
fn head() -> TreeModel {
    let mesh =
        merged(Role::Bark, ATLAS, &[([-0.15, 2.15, 0.05], [0.15, 3.05, 0.26]), ([-0.1, 2.8, 0.26], [0.1, 3.0, 0.43])]);
    TreeModel { levels: vec![Level { parts: vec![mesh] }], textures: HashMap::from([(ATLAS, picture(64, 64))]) }
}

fn all_signs() -> HashMap<&'static str, TreeModel> {
    SIGNS.iter().map(|sign_entry| (sign_entry.key, sign())).collect()
}

/// What `place::exact` of the game accepts: the model's box inside the old box grown by half of `spread - 1` of its size and 5 cm.
fn inside_what_the_game_accepts(low: [f32; 3], high: [f32; 3], old: &Box3) -> bool {
    (0..3).all(|axis| {
        let room = 0.5 * (build::SPREAD - 1.0) * (old.high[axis] - old.low[axis]) + 0.05;
        low[axis] >= old.low[axis] - room && high[axis] <= old.high[axis] + room
    })
}

fn box_of(solid: &blackbox_solid::Solid) -> ([f32; 3], [f32; 3]) {
    let mut low = [f32::MAX; 3];
    let mut high = [f32::MIN; 3];
    for vertex in &solid.vertices {
        for axis in 0..3 {
            low[axis] = low[axis].min(vertex.position[axis]);
            high[axis] = high[axis].max(vertex.position[axis]);
        }
    }
    (low, high)
}

#[test]
fn the_tables_name_every_solid_once_and_every_sign_they_use_exists() {
    let mut seen = HashSet::new();
    let targets = SIGN_SWAPS
        .iter()
        .map(|s| s.target)
        .chain(ARM_SWAPS.iter().map(|s| s.target))
        .chain(HEAD_SWAPS.iter().map(|s| s.target));
    for target in targets {
        assert!(seen.insert(target), "{target} is replaced twice");
    }
    for swap in &SIGN_SWAPS {
        assert!(SIGNS.iter().any(|sign| sign.key == swap.sign), "{} names no sign", swap.target);
        assert!(swap.old.high[2] - swap.old.low[2] > 0.5, "{} has no height", swap.target);
    }
    let boxes = ARM_SWAPS
        .iter()
        .map(|s| &s.old)
        .chain(HEAD_SWAPS.iter().map(|s| &s.old))
        .chain(SIGN_SWAPS.iter().map(|s| &s.old));
    for old in boxes {
        assert!((0..3).all(|axis| old.low[axis] < old.high[axis]), "{old:?}");
    }
    let mut keys = HashSet::new();
    assert!(SIGNS.iter().all(|sign| keys.insert(sign.key)), "two signs share a key");
}

#[test]
fn the_signs_pack_has_a_model_for_every_swap_inside_the_box_the_game_accepts() {
    let pack = build::signs(&all_signs(), 1024).unwrap();
    assert_eq!(pack.swaps, SIGN_SWAPS.len());
    let solids = read_solids(&pack.geometry).unwrap();
    assert_eq!(solids.len(), SIGN_SWAPS.len());
    let set = parse_replace_set("signs-mw2012", &pack.manifest).unwrap();
    assert_eq!((set.kind.as_str(), set.value.as_str()), ("road_signs", "mw2012"));
    assert_eq!(set.models.len(), SIGN_SWAPS.len());
    for (entry, swap) in set.models.iter().zip(&SIGN_SWAPS) {
        let solid = solids.iter().find(|solid| solid.name == entry.model).unwrap();
        let (low, high) = box_of(solid);
        assert!(inside_what_the_game_accepts(low, high, &swap.old), "{}: {low:?} {high:?}", swap.target);
        assert!(high[2] <= swap.old.high[2] + 0.05, "{}: no taller than the old sign", swap.target);
        assert!(solid.groups.iter().all(|group| group.effect_id == 0 && group.flags == 0x4000));
        assert!(solid.markers.is_empty());
    }
    let hashes: Vec<u32> =
        read_texture_packs(&pack.textures).unwrap().into_iter().flat_map(|p| p.textures).map(|t| t.name_hash).collect();
    assert!(hashes.contains(&bstring_hash(&texture_name(PLATE))));
    for solid in &solids {
        assert!(solid.texture_hashes.iter().all(|hash| hashes.contains(hash)));
    }
}

#[test]
fn a_sign_shows_its_picture_towards_minus_y_like_the_games_own() {
    let pack = build::signs(&all_signs(), 1024).unwrap();
    let solid = read_solids(&pack.geometry).unwrap().into_iter().next().unwrap();
    // The plate is the group with the larger area of faces looking along y; its triangles must look towards -y.
    let plate = &solid.groups[0];
    let first = plate.first_index as usize;
    let [a, b, c] =
        [0, 1, 2].map(|k| solid.vertices[(solid.indices[first + k] as u32 + plate.base_vertex) as usize].position);
    let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
    let ny = u[2] * v[0] - u[0] * v[2];
    assert!(ny < 0.0, "the plate's winding looks towards -y: {ny}");
}

#[test]
fn the_lights_pack_replaces_the_arms_and_the_heads_each_in_its_own_box() {
    let pack = build::lights(&arm(), &head(), 1024).unwrap();
    assert_eq!(pack.swaps, ARM_SWAPS.len() + HEAD_SWAPS.len());
    let solids = read_solids(&pack.geometry).unwrap();
    let set = parse_replace_set("lights-mw2012", &pack.manifest).unwrap();
    assert_eq!((set.kind.as_str(), set.value.as_str()), ("traffic_lights", "mw2012"));
    let olds: Vec<(&str, Box3)> =
        ARM_SWAPS.iter().map(|s| (s.target, s.old)).chain(HEAD_SWAPS.iter().map(|s| (s.target, s.old))).collect();
    assert_eq!(set.models.len(), olds.len());
    for (entry, (target, old)) in set.models.iter().zip(&olds) {
        let solid = solids.iter().find(|solid| solid.name == entry.model).unwrap();
        let (low, high) = box_of(solid);
        assert!(inside_what_the_game_accepts(low, high, old), "{target}: {low:?} {high:?} in {old:?}");
        let is_head = target.contains("LIGHTC") || target.contains("LIGHTD");
        assert_eq!(solid.markers.len(), usize::from(is_head), "{target}: only a head carries the red flare");
    }
}

#[test]
fn a_mast_arm_loses_its_heads_and_fills_the_pole_of_the_game() {
    let pack = build::lights(&arm(), &head(), 1024).unwrap();
    let solids = read_solids(&pack.geometry).unwrap();
    let set = parse_replace_set("lights-mw2012", &pack.manifest).unwrap();
    let a = ARM_SWAPS.iter().position(|s| s.target == "XO_TRAFFICLIGHTA_1B_00").unwrap();
    let solid = solids.iter().find(|solid| solid.name == set.models[a].model).unwrap();
    let (low, high) = box_of(solid);
    let old = ARM_SWAPS[a].old;
    assert!((high[2] - old.high[2]).abs() < 1e-3 && (low[1] - old.low[1]).abs() < 1e-3, "{low:?} {high:?}");
    // The pole and the arm are two boxes of 8 triangles; the head that hung from the arm is gone.
    let triangles: usize = solid.groups.iter().map(|group| group.num_triangles as usize).sum();
    assert_eq!(triangles, 16);
}

#[test]
fn a_signal_head_hangs_below_its_origin_with_a_hanger_and_a_red_flare_on_the_front() {
    let pack = build::lights(&arm(), &head(), 1024).unwrap();
    let solids = read_solids(&pack.geometry).unwrap();
    let set = parse_replace_set("lights-mw2012", &pack.manifest).unwrap();
    let d = HEAD_SWAPS.iter().position(|s| s.target == "XO_TRAFFICLIGHTD_1B_00").unwrap();
    let entry = &set.models[ARM_SWAPS.len() + d];
    let solid = solids.iter().find(|solid| solid.name == entry.model).unwrap();
    let (low, high) = box_of(solid);
    assert!(
        high[2] > 0.0 && high[2] <= 0.3 && low[2] < -1.0,
        "hangs from its origin, the hanger reaching a little above it: {low:?} {high:?}"
    );
    assert_eq!(solid.groups.len(), 2, "the head and the hanger");
    let marker = solid.markers[0];
    assert_eq!(marker.int_param, 4, "a red blinking flare");
    let at = marker.translation();
    assert!(at[0] > 0.0 && at[2] > table::HANG_D.centre, "in front of the head and above its middle lens: {at:?}");
    // The hanger is a flat colour, not the head's picture.
    assert_eq!(solid.texture_hashes.len(), 2);
}

#[test]
fn the_manifests_name_their_files_and_the_fit_the_models_are_authored_for() {
    let text =
        build::manifest_text(&[("XS_A", "MW12_S00".to_owned())], "road_signs", "nfsmw2012-road-signs", "SIGNS/MW2012");
    let set = parse_replace_set("signs-mw2012", &text).unwrap();
    assert_eq!(set.geometry.as_deref(), Some("SIGNS/MW2012/GEOMETRY.BIN"));
    assert_eq!(set.textures.as_deref(), Some("SIGNS/MW2012/TEXTURES.BIN"));
    assert_eq!(set.kind_label.as_deref(), Some("nfsmw2012-road-signs"));
    assert_eq!(set.value_label, "NFS Most Wanted 2012");
    assert_eq!(set.models.len(), 1);
    assert!(text.contains("fit = \"exact\""));
}

#[test]
fn a_model_that_is_missing_costs_only_its_swaps() {
    let mut signs = all_signs();
    signs.remove("STOP");
    let pack = build::signs(&signs, 1024).unwrap();
    assert_eq!(pack.swaps, SIGN_SWAPS.len() - 1, "the stop sign A is left out, the rest stands");
    assert!(build::signs(&HashMap::new(), 1024).is_err(), "no sign at all is no pack");
}
