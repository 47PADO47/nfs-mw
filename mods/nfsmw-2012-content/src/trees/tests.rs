//! The trees variant on authored data: a tree of two meshes (a trunk quad and a card of leaves) in the 2012 engine's space, and
//! the lists of species and of the game's solids. No game data.

use std::collections::{HashMap, HashSet};

use blackbox_hash::bstring_hash;
use blackbox_modding::content::{SwapFit, parse_model_swap_profile};
use blackbox_solid::read_solids;
use blackbox_tpk::{AlphaUsage, PixelFormat, read_texture_packs};
use chameleon_gfx::{PixelFormat as SourceFormat, Texture, mip_size};

use super::build::{self, texture_name};
use super::read::{Level, Part, Role, TreeModel};
use super::species::{SPECIES, solid_name};
use super::stock::{self, FAMILIES};

const LEAF: u64 = 0x0100_0000_0010_4f52;
const BARK: u64 = 0x0100_0000_0010_4f49;

fn picture(width: u32, height: u32, mips: u32) -> Texture {
    let size: usize = (0..mips).map(|level| mip_size(SourceFormat::Bc1, width, height, level).unwrap()).sum();
    Texture {
        width,
        height,
        depth: 1,
        array_size: 1,
        mip_levels: mips,
        dxgi_format: 71,
        format: SourceFormat::Bc1,
        data: vec![0x55; size],
    }
}

fn quad(corners: [[f32; 3]; 4], role: Role, texture: u64) -> Part {
    Part {
        role,
        positions: corners.to_vec(),
        uvs: vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
        texture,
    }
}

/// A tree in the 2012 engine's space (x left, y up, z forward): a trunk 4 metres tall and a card of leaves from 3 to 10 metres.
fn tree(levels: usize) -> TreeModel {
    let trunk = quad([[-0.2, 0.0, 0.0], [0.2, 0.0, 0.0], [0.2, 4.0, 0.0], [-0.2, 4.0, 0.0]], Role::Bark, BARK);
    let leaves = quad([[-3.0, 3.0, 0.0], [3.0, 3.0, 0.0], [3.0, 10.0, 0.0], [-3.0, 10.0, 0.0]], Role::Foliage, LEAF);
    let level = Level { parts: vec![trunk, leaves] };
    let textures = HashMap::from([(LEAF, picture(64, 64, 7)), (BARK, picture(32, 32, 6))]);
    TreeModel { levels: vec![level; levels], textures }
}

#[test]
fn a_species_becomes_a_solid_per_level_in_2005_space_with_the_game_s_effects() {
    let pack = build::build(&[(&SPECIES[0], tree(2))], 1024).unwrap();
    assert_eq!(pack.species, [("BROADLEAF_S", 2)]);
    let solids = read_solids(&pack.geometry).unwrap();
    let mut names: Vec<&str> = solids.iter().map(|solid| solid.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["MW12_BROADLEAF_S_L0", "MW12_BROADLEAF_S_L1"]);
    let solid = solids.iter().find(|solid| solid.name == solid_name("BROADLEAF_S", 0)).unwrap();
    let effects: Vec<u32> = solid.groups.iter().map(|group| group.effect_id).collect();
    assert_eq!(effects, [0, 6], "the trunk is the world shader, the leaves the tree shader");
    assert_eq!(solid.groups[0].flags, 0x4000);
    assert_eq!(solid.groups[1].flags, 0x4008);
    assert_eq!(solid.texture_hashes, vec![bstring_hash(&texture_name(BARK)), bstring_hash(&texture_name(LEAF))]);
    // x left, y up, z forward becomes x forward, y left, z up: the up of the tree is z.
    let top = solid.vertices.iter().map(|v| v.position[2]).fold(f32::MIN, f32::max);
    let foot = solid.vertices.iter().map(|v| v.position[2]).fold(f32::MAX, f32::min);
    assert_eq!((top, foot), (10.0, 0.0));
    assert_eq!(solid.vertices[0].position, [0.0, -0.2, 0.0], "the first trunk corner: (x, y, z) goes to (z, x, y)");
}

#[test]
fn the_leaf_cards_have_normals_away_from_the_crown_and_a_grey_that_grows_lighter_upwards() {
    let pack = build::build(&[(&SPECIES[0], tree(1))], 1024).unwrap();
    let solid = read_solids(&pack.geometry).unwrap().pop().unwrap();
    let leaves = &solid.vertices[solid.groups[0].num_vertices as usize..];
    for vertex in leaves {
        let length = vertex.normal.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((length - 1.0).abs() < 1e-3, "{:?}", vertex.normal);
    }
    let low = leaves.iter().find(|v| v.position[2] == 3.0).unwrap();
    let high = leaves.iter().find(|v| v.position[2] == 10.0).unwrap();
    assert_eq!((low.color_bgra, high.color_bgra), ([47, 47, 47, 255], [70, 70, 70, 255]));
    assert!(low.normal[2] < 0.0 && high.normal[2] > 0.0, "the bottom of the crown faces down, the top up");
}

#[test]
fn the_textures_are_named_for_the_2012_resource_and_leaves_are_two_sided_cut_outs() {
    let pack = build::build(&[(&SPECIES[0], tree(1))], 1024).unwrap();
    let textures = read_texture_packs(&pack.textures).unwrap().remove(0).textures;
    let leaf = textures.iter().find(|t| t.name_hash == bstring_hash(&texture_name(LEAF))).unwrap();
    assert_eq!(texture_name(LEAF), "MW12_T104F52");
    assert_eq!((leaf.alpha_usage, leaf.flags & 2, leaf.format), (AlphaUsage::PunchThrough, 2, PixelFormat::Dxt1));
    let bark = textures.iter().find(|t| t.name_hash == bstring_hash(&texture_name(BARK))).unwrap();
    assert_eq!((bark.alpha_usage, bark.tilable_uv), (AlphaUsage::None, 3));
    assert_eq!(textures.len(), 2);
}

#[test]
fn a_species_with_a_texture_the_old_engine_cannot_draw_is_left_out_and_the_others_stay() {
    let mut bad = tree(1);
    bad.textures.get_mut(&BARK).unwrap().format = SourceFormat::Bc5;
    let pack = build::build(&[(&SPECIES[0], bad), (&SPECIES[1], tree(1))], 1024).unwrap();
    assert_eq!(pack.species, [("BROADLEAF_M", 1)]);
    assert!(build::build(&[(&SPECIES[0], TreeModel { levels: Vec::new(), textures: HashMap::new() })], 1024).is_err());
}

#[test]
fn the_profile_swaps_every_stock_solid_of_the_species_built_and_the_game_accepts_it() {
    let text = build::profile_text(&[("BROADLEAF_M", 4), ("BUSH", 4), ("POPLAR", 2)]);
    let profile = parse_model_swap_profile(&text).unwrap();
    assert_eq!(profile.geometry, "TREES/MW2012/GEOMETRY.BIN");
    assert_eq!(profile.textures, "TREES/MW2012/TEXTURES.BIN");
    let swap = |solid: &str| profile.swaps.iter().find(|swap| swap.solid == solid).unwrap_or_else(|| panic!("{solid}"));
    // A single tree: the coarsest level for the far solid, the finest for the near one.
    assert_eq!(
        (swap("XT_BUSHMULLD_1B_RB_00").model.as_str(), swap("XT_BUSHMULLD_1B_RB_00").fit),
        ("MW12_BUSH_L0", SwapFit::Height)
    );
    assert_eq!(swap("XT_BUSHMULLD_1Z_RB_00").model, "MW12_BUSH_L3");
    assert_eq!(swap("XT_CT_POPLARD_1C_JG_00").model, "MW12_POPLAR_L1");
    // A row takes a coarser level than a single tree.
    assert_eq!(
        (swap("XT_CT_TREELINE1_1B_00").model.as_str(), swap("XT_CT_TREELINE1_1B_00").fit),
        ("MW12_BROADLEAF_M_L1", SwapFit::Row)
    );
    assert!(
        profile.swaps.iter().all(|swap| !swap.solid.starts_with("XT_CEDARM")),
        "a species that is not built has no swap"
    );
}

#[test]
fn a_profile_of_no_species_has_no_swap_and_the_game_refuses_it() {
    assert!(parse_model_swap_profile(&build::profile_text(&[])).is_err());
}

#[test]
fn the_species_are_distinct_and_in_the_world_bundles() {
    let keys: HashSet<&str> = SPECIES.iter().map(|species| species.key).collect();
    let models: HashSet<u64> = SPECIES.iter().map(|species| species.model).collect();
    assert_eq!((keys.len(), models.len()), (SPECIES.len(), SPECIES.len()));
    for species in &SPECIES {
        assert!(species.bundle.starts_with("HAWAII/") && species.bundle.ends_with(".BNDL"), "{}", species.key);
        assert!(species.key.bytes().all(|b| b.is_ascii_uppercase() || b == b'_'), "{}", species.key);
        assert!(solid_name(species.key, 3).len() <= 63, "a solid name the profile accepts");
    }
}

#[test]
fn every_family_names_a_species_and_every_solid_name_is_listed_once() {
    let keys: HashSet<&str> = SPECIES.iter().map(|species| species.key).collect();
    let mut seen = HashSet::new();
    for family in FAMILIES {
        assert!(keys.contains(family.species), "{}", family.stem);
        for (name, code) in family.names() {
            assert!(seen.insert(name.clone()), "{name} is listed twice");
            assert!(name.contains(&format!("_{code}_")), "{name}");
            assert!(name.len() <= 63);
        }
    }
    assert!(seen.contains("XT_CEDARM_1B_JG_00") && seen.contains("XT_CT_TREELINE1_1B_00"));
}

#[test]
fn the_level_a_solid_takes_follows_its_letter_and_whether_it_is_a_row() {
    assert_eq!(
        [("1A", false), ("1B", false), ("1C", false), ("1Z", false)].map(|(code, row)| stock::level(code, row, 4)),
        [0, 0, 1, 3]
    );
    assert_eq!(
        [("1A", true), ("1B", true), ("1C", true), ("1Z", true)].map(|(code, row)| stock::level(code, row, 4)),
        [1, 1, 2, 3]
    );
    assert_eq!(stock::level("1Z", false, 2), 1, "a model with two levels: the coarsest is the second");
    assert_eq!(stock::level("1C", true, 2), 1, "never past the last level");
    assert_eq!(stock::level("2B", false, 1), 0);
}
