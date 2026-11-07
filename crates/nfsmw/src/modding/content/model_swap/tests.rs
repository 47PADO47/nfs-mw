//! Fitting and swapping with authored solids: a tree that is a trunk quad and a foliage card, and boxes that stand for what
//! it replaces. No game data.

use std::sync::Arc;

use blackbox_hash::bstring_hash;
use blackbox_modding::content::{ModelSwapProfile, SwapFit, parse_model_swap_profile};
use blackbox_solid::{GroupSource, Solid, SolidSource, Vertex, read_solids, write_geometry_pack};
use blackbox_tpk::{AlphaUsage, PixelFormat, Texture};
use nfsmw_data::world::SectionData;

use super::fit::{
    Frame, MAX_SCALE, MIN_SCALE, Prepared, ROW_MAX_COPIES, ROW_MAX_SCALE, fit_prepared, footprint, placements,
};
use super::pack::world_copy_base;
use super::{Pack, TEST_LOCK, active, rewrite, set_active};

/// `model` fitted into the local solid `stock`.
fn fit(model: &Solid, stock: &Solid, how: SwapFit) -> anyhow::Result<Solid> {
    fit_prepared(&Prepared::new(model)?, stock, how, Frame::Local)
}

fn vertex(position: [f32; 3], uv: [f32; 2]) -> Vertex {
    Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: [128; 4], uv }
}

fn quad(corners: [[f32; 3]; 4]) -> Vec<Vertex> {
    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    corners.iter().zip(uvs).map(|(corner, uv)| vertex(*corner, uv)).collect()
}

fn write(source: SolidSource) -> Solid {
    read_solids(&write_geometry_pack(&[source]).unwrap()).unwrap().pop().unwrap()
}

/// A tree: a trunk quad (effect 0) and a foliage card (effect 6) from 30 percent of the height up, `half_width` to each
/// side of the origin in x.
fn tree(name: &str, texture: &str, height: f32, half_width: f32, flags: u16) -> Solid {
    let trunk_top = height * 0.4;
    let trunk = GroupSource {
        effect_id: 0,
        flags: 0x4000,
        ..GroupSource::new(
            quad([[-0.2, 0.0, 0.0], [0.2, 0.0, 0.0], [0.2, 0.0, trunk_top], [-0.2, 0.0, trunk_top]]),
            vec![0, 1, 2, 0, 2, 3],
        )
    };
    let (low, high) = (height * 0.3, height);
    let leaves = GroupSource {
        effect_id: 6,
        flags: 0x4008,
        ..GroupSource::new(
            quad([[-half_width, 0.0, low], [half_width, 0.0, low], [half_width, 0.0, high], [-half_width, 0.0, high]]),
            vec![0, 1, 2, 0, 2, 3],
        )
    };
    let mut source = SolidSource::new(name, vec![trunk, leaves]);
    source.flags = flags;
    source.texture_hashes = vec![bstring_hash(texture)];
    write(source)
}

/// A solid with only the bounds `low` to `high`: a quad across them.
fn bounds_box(name: &str, low: [f32; 3], high: [f32; 3]) -> Solid {
    let corners = [low, [high[0], low[1], low[2]], high, [low[0], high[1], high[2]]];
    let mut source = SolidSource::new(name, vec![GroupSource::new(quad(corners), vec![0, 1, 2, 0, 2, 3])]);
    source.texture_hashes = vec![bstring_hash("STOCKTEX")];
    write(source)
}

fn highest(solid: &Solid) -> f32 {
    solid.vertices.iter().map(|v| v.position[2]).fold(f32::MIN, f32::max)
}

fn lowest(solid: &Solid) -> f32 {
    solid.vertices.iter().map(|v| v.position[2]).fold(f32::MAX, f32::min)
}

#[test]
fn a_replacement_takes_the_stock_solids_identity_and_its_height() {
    let stock = tree("XT_STOCK", "STOCKTEX", 20.0, 7.0, 0x80);
    let model = tree("MODEL", "LEAF", 10.0, 3.0, 0x40);
    let fitted = fit(&model, &stock, SwapFit::Height).unwrap();
    assert_eq!((fitted.name.as_str(), fitted.name_hash), ("XT_STOCK", bstring_hash("XT_STOCK")));
    assert_eq!(fitted.flags, 0x80, "the sway bit of the stock solid, not the flags of the model");
    assert_eq!(fitted.transform, stock.transform);
    assert_eq!((fitted.num_polys, fitted.density), (stock.num_polys, stock.density));
    assert_eq!(fitted.texture_hashes, vec![bstring_hash("LEAF")]);
    assert_eq!(fitted.groups.len(), 2);
    let top = highest(&fitted);
    assert!((top - highest(&stock)).abs() < 0.1, "the top: {top} against {}", highest(&stock));
    assert!((lowest(&fitted) - lowest(&stock)).abs() < 0.1, "the base stands on the floor of the stock solid");
}

#[test]
fn a_replacement_is_a_solid_the_loader_would_have_read() {
    let stock = tree("XT_STOCK", "STOCKTEX", 20.0, 7.0, 0);
    let fitted = fit(&tree("MODEL", "LEAF", 10.0, 3.0, 0), &stock, SwapFit::Height).unwrap();
    let effects: Vec<u32> = fitted.groups.iter().map(|group| group.effect_id).collect();
    assert_eq!(effects, [0, 6], "the groups keep their effects: foliage stays a tree effect");
    let vertex_count: usize = fitted.groups.iter().map(|group| group.num_vertices as usize).sum();
    assert_eq!(vertex_count, fitted.vertices.len());
    assert!(fitted.bounds_max[2] >= highest(&fitted) && fitted.bounds_min[2] <= lowest(&fitted));
}

#[test]
fn a_model_is_set_on_the_floor_of_the_solid_it_replaces() {
    let stock = bounds_box("XT_BOX", [-2.0, -2.0, 5.0], [2.0, 2.0, 25.0]);
    let fitted = fit(&tree("MODEL", "LEAF", 10.0, 3.0, 0), &stock, SwapFit::Height).unwrap();
    assert!((lowest(&fitted) - 5.0).abs() < 0.1, "the floor of the stock solid is z = 5: {}", lowest(&fitted));
    assert!((highest(&fitted) - 25.0).abs() < 0.1);
}

#[test]
fn a_row_sets_copies_along_the_longer_side_and_scales_them_no_further_than_one_and_a_half_times() {
    let model = tree("MODEL", "LEAF", 10.0, 3.0, 0);
    let stock = bounds_box("XT_ROW", [-30.0, -5.0, 0.0], [30.0, 5.0, 20.0]);
    let fitted = fit(&model, &stock, SwapFit::Row).unwrap();
    assert_eq!(fitted.groups.len(), 2 * ROW_MAX_COPIES, "60 metres of 7 metre crowns is more than the most copies");
    let top = highest(&fitted);
    assert!((top - ROW_MAX_SCALE * 10.0).abs() < 0.3, "scaled by {ROW_MAX_SCALE}, not to the stock height: {top}");
    for vertex in &fitted.vertices {
        assert!(vertex.position[0].abs() < 31.5, "inside the long side: {:?}", vertex.position);
        assert!(vertex.position[1].abs() < 9.6, "near the middle of the short side: {:?}", vertex.position);
    }
}

#[test]
fn a_short_row_has_one_copy_and_the_other_axis_is_used_when_it_is_longer() {
    let model_bounds = ([-3.0, -0.2, 0.0], [3.0, 0.2, 10.0]);
    let short = placements(SwapFit::Row, model_bounds, ([-3.0, -3.0, 0.0], [3.0, 3.0, 10.0]));
    assert_eq!(short.len(), 1);
    assert_eq!(short[0].offset, [0.0, 0.0, 0.0]);
    let along_y = placements(SwapFit::Row, model_bounds, ([-4.0, -20.0, 0.0], [4.0, 20.0, 10.0]));
    assert!(along_y.len() > 1);
    assert!(along_y.iter().all(|at| at.offset[0] == 0.0), "the copies stand on the middle of the short side");
    assert!(along_y.windows(2).all(|pair| pair[0].offset[1] < pair[1].offset[1]), "along y, in order");
    assert!(along_y.windows(2).all(|pair| pair[0].yaw != pair[1].yaw), "no two copies are alike");
}

#[test]
fn a_height_fit_is_one_placement_with_the_scale_kept_in_range() {
    let model_bounds = ([-1.0, -1.0, 0.0], [1.0, 1.0, 1.0]);
    let tall = placements(SwapFit::Height, model_bounds, ([-1.0, -1.0, 0.0], [1.0, 1.0, 100.0]));
    assert_eq!(tall.len(), 1);
    assert_eq!(tall[0].scale, MAX_SCALE);
    let flat =
        placements(SwapFit::Height, ([-1.0, -1.0, 0.0], [1.0, 1.0, 100.0]), ([-1.0, -1.0, 0.0], [1.0, 1.0, 1.0]));
    assert_eq!(flat[0].scale, MIN_SCALE);
}

#[test]
fn a_solid_without_height_is_an_error() {
    let flat = bounds_box("XT_FLAT", [-1.0, -1.0, 2.0], [1.0, 1.0, 2.0]);
    let model = tree("MODEL", "LEAF", 10.0, 3.0, 0);
    // The writer pads every bound by a centimetre, so a flat quad is still 2 cm tall: make the check bite with a real zero.
    let mut thin = flat.clone();
    thin.bounds_min[2] = 2.0;
    thin.bounds_max[2] = 2.0;
    assert!(fit(&model, &thin, SwapFit::Height).is_err(), "a stock solid with no height");
}

fn texture(name: &str) -> Texture {
    Texture {
        name: name.to_owned(),
        name_hash: bstring_hash(name),
        width: 4,
        height: 4,
        mip_levels: 1,
        format: PixelFormat::Dxt1,
        compression_type: 0x22,
        alpha_usage: AlphaUsage::PunchThrough,
        alpha_sorting: false,
        alpha_blend: 0,
        flags: 2,
        tilable_uv: 0,
        data: vec![0; 8],
        palette: Vec::new(),
    }
}

fn section(solids: Vec<Solid>) -> SectionData {
    SectionData {
        index: 0,
        solids,
        textures: vec![texture("STOCKTEX")],
        decoded: vec![None],
        anims: Vec::new(),
        scrolls: Vec::new(),
        scenery: Vec::new(),
        spawners: Vec::new(),
        emitters: Vec::new(),
        collision: Vec::new(),
        prepared: None,
    }
}

fn profile(solid: &str, model: &str) -> ModelSwapProfile {
    parse_model_swap_profile(&format!(
        "format = 1\ngeometry = \"TREES/X/GEOMETRY.BIN\"\ntextures = \"TREES/X/TEXTURES.BIN\"\n[[swap]]\nsolid = \"{solid}\"\nmodel = \"{model}\"\n"
    ))
    .unwrap()
}

fn pack() -> Pack {
    let models = vec![tree("MW12_TREE_L0", "MW12_LEAF", 10.0, 3.0, 0)];
    Pack::build(&profile("XT_STOCK", "MW12_TREE_L0"), models, vec![texture("MW12_LEAF"), texture("UNUSED")])
}

#[test]
fn a_section_gets_the_models_for_its_solids_and_no_copy_of_the_textures() {
    let pack = pack();
    assert_eq!(pack.swaps(), 1);
    let stock = tree("XT_STOCK", "STOCKTEX", 20.0, 7.0, 0x80);
    let other = tree("XT_OTHER", "STOCKTEX", 5.0, 2.0, 0);
    let before = other.clone();
    let mut data = section(vec![stock, other]);
    pack.apply(&mut data);
    assert_eq!(data.solids[0].texture_hashes, vec![bstring_hash("MW12_LEAF")], "the solid of the stock name");
    assert_eq!(data.solids[0].flags, 0x80);
    assert_eq!(data.solids[1], before, "a solid the pack has no model for is untouched");
    let names: Vec<&str> = data.textures.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["STOCKTEX"], "the textures stay in the pack: the world uploads them once, into the shared sets");
    assert_eq!(data.decoded.len(), data.textures.len());
    let packed: Vec<&str> = pack.textures().iter().map(|t| t.name.as_str()).collect();
    assert_eq!(packed, ["MW12_LEAF", "UNUSED"]);
}

#[test]
fn a_section_with_nothing_to_replace_is_left_alone() {
    let mut data = section(vec![tree("XT_OTHER", "STOCKTEX", 5.0, 2.0, 0)]);
    pack().apply(&mut data);
    assert_eq!(data.textures.len(), 1);
}

#[test]
fn the_name_of_a_world_copy_gives_the_solid_it_was_made_of() {
    assert_eq!(world_copy_base("XT_CEDARM_1B_JG_00_DEINST12_T5"), Some("XT_CEDARM_1B_JG_00"));
    assert_eq!(world_copy_base("XT_CT_TREELINE1_1B_00_DEINST0_X0"), Some("XT_CT_TREELINE1_1B_00"));
    assert_eq!(world_copy_base("XT_CEDARM_1B_JG_00"), None);
    assert_eq!(world_copy_base("XT_X_DEINSTALL"), None, "a digit follows the mark");
}

/// A world copy of a tree standing at `at`: the tree's vertices moved to there.
fn world_tree(name: &str, at: [f32; 2], height: f32) -> Solid {
    let mut source = SolidSource::from_solid(&tree("T", "STOCKTEX", height, 3.0, 0x80)).unwrap();
    for group in &mut source.groups {
        for vertex in &mut group.vertices {
            vertex.position[0] += at[0];
            vertex.position[1] += at[1];
            vertex.position[2] += 4.0;
        }
    }
    let mut renamed = SolidSource::new(name, source.groups);
    renamed.flags = 0x80;
    renamed.texture_hashes = source.texture_hashes;
    write(renamed)
}

#[test]
fn a_world_copy_is_replaced_where_it_stands_and_keeps_its_own_name() {
    let pack = pack();
    let copy = world_tree("XT_STOCK_DEINST3_T7", [500.0, -300.0], 20.0);
    let mut data = section(vec![copy.clone()]);
    pack.apply(&mut data);
    let fitted = &data.solids[0];
    assert_eq!(fitted.name, "XT_STOCK_DEINST3_T7", "the copy keeps its name: the scenery refers to it by that hash");
    assert_eq!(fitted.texture_hashes, vec![bstring_hash("MW12_LEAF")]);
    assert!((lowest(fitted) - lowest(&copy)).abs() < 0.1, "it stands on the floor of the copy");
    assert!((highest(fitted) - highest(&copy)).abs() < 0.1, "it is as tall as the copy");
    let (low, high) = (fitted.bounds_min, fitted.bounds_max);
    assert!(low[0] > 480.0 && high[0] < 520.0, "near the copy's x, not near the origin: {low:?} {high:?}");
    assert!(low[1] > -320.0 && high[1] < -280.0, "near the copy's y: {low:?} {high:?}");
    assert!(pack.fitted.lock().unwrap().is_empty(), "nothing of a world copy is kept in the cache");
}

#[test]
fn a_row_in_world_space_runs_along_the_way_the_copy_runs() {
    let model = ([-3.0, -0.2, 0.0], [3.0, 0.2, 10.0]);
    // A long thin strip on the diagonal: ten metres thick, a hundred long, 45 degrees from the x axis.
    let corners = [[0.0, 0.0], [70.0, 70.0], [77.0, 63.0], [7.0, -7.0]];
    let vertices = corners.iter().map(|c| vertex([c[0], c[1], 0.0], [0.0, 0.0]));
    let top = corners.iter().map(|c| vertex([c[0], c[1], 20.0], [0.0, 0.0]));
    let mut copy = bounds_box("XT_ROW_DEINST1_T1", [0.0, -7.0, 0.0], [77.0, 70.0, 20.0]);
    copy.vertices = vertices.chain(top).collect();
    let plan = footprint(&copy);
    assert!((plan.angle - std::f32::consts::FRAC_PI_4).abs() < 0.05, "along the diagonal: {}", plan.angle);
    assert!((plan.length - 99.0).abs() < 3.0, "about 100 metres: {}", plan.length);
    assert!((plan.centre[0] - 38.5).abs() < 2.0 && (plan.centre[1] - 31.5).abs() < 2.0, "{:?}", plan.centre);
    let copies = super::fit::world_placements(SwapFit::Row, model, (copy.bounds_min, copy.bounds_max), &plan);
    assert_eq!(copies.len(), ROW_MAX_COPIES);
    let (first, last) = (copies[0].offset, copies[ROW_MAX_COPIES - 1].offset);
    assert!(last[0] > first[0] && last[1] > first[1], "both axes grow along the diagonal: {first:?} {last:?}");
    assert!(((last[1] - first[1]) / (last[0] - first[0]) - 1.0).abs() < 0.2, "on the diagonal");
}

#[test]
fn a_world_single_tree_is_placed_at_the_middle_of_its_footprint() {
    let copy = world_tree("XT_STOCK_DEINST3_T7", [10.0, 20.0], 20.0);
    let plan = footprint(&copy);
    assert!((plan.centre[0] - 10.0).abs() < 0.5 && (plan.centre[1] - 20.0).abs() < 0.5, "{:?}", plan.centre);
    let bounds = ([-3.0, -0.2, 0.0], [3.0, 0.2, 10.0]);
    let at = super::fit::world_placements(SwapFit::Height, bounds, (copy.bounds_min, copy.bounds_max), &plan);
    assert_eq!(at.len(), 1);
    assert!((at[0].offset[2] - copy.bounds_min[2]).abs() < 1e-3);
}

#[test]
fn the_same_stock_solid_in_two_sections_is_fitted_the_same() {
    let pack = pack();
    let mut first = section(vec![tree("XT_STOCK", "STOCKTEX", 20.0, 7.0, 0)]);
    let mut second = section(vec![tree("XT_STOCK", "STOCKTEX", 20.0, 7.0, 0)]);
    pack.apply(&mut first);
    pack.apply(&mut second);
    assert_eq!(first.solids[0], second.solids[0]);
}

#[test]
fn a_swap_whose_model_or_texture_is_missing_is_left_out() {
    let models = vec![tree("MW12_TREE_L0", "MW12_LEAF", 10.0, 3.0, 0)];
    let nobody = Pack::build(&profile("XT_STOCK", "NO_SUCH_MODEL"), models.clone(), vec![texture("MW12_LEAF")]);
    assert_eq!(nobody.swaps(), 0);
    let bare = Pack::build(&profile("XT_STOCK", "MW12_TREE_L0"), models, vec![texture("OTHER")]);
    assert_eq!(bare.swaps(), 0, "the texture of the model is not in the texture pack");
}

#[test]
fn the_pack_in_effect_rewrites_sections_and_none_leaves_them() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    set_active(Some(Arc::new(pack())));
    assert!(active().is_some());
    let mut data = section(vec![tree("XT_STOCK", "STOCKTEX", 20.0, 7.0, 0)]);
    rewrite(&mut data);
    assert_eq!(data.solids[0].texture_hashes, vec![bstring_hash("MW12_LEAF")]);
    set_active(None);
    assert!(active().is_none());
    let mut untouched = section(vec![tree("XT_STOCK", "STOCKTEX", 20.0, 7.0, 0)]);
    rewrite(&mut untouched);
    assert_eq!(untouched.solids[0].texture_hashes, vec![bstring_hash("STOCKTEX")]);
}
