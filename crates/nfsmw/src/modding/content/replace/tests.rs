//! Fitting and swapping with authored solids: a tree that is a trunk quad and a foliage card, and boxes that stand for what
//! it replaces. No game data.

use std::sync::Arc;

use blackbox_hash::bstring_hash;
use blackbox_modding::content::{FitLimits, ModelSwapProfile, SwapFit, from_model_swap, parse_model_swap_profile};
use blackbox_solid::{GroupSource, Solid, SolidSource, Vertex, read_solids, write_geometry_pack};
use blackbox_tpk::{AlphaUsage, PixelFormat, Texture};
use nfsmw_data::world::SectionData;

use super::active::{LoadedSet, world_copy_base};
use super::fit::{Frame, Prepared, fit_prepared};
use super::place::{MAX_COPIES as ROW_COPIES_AT_MOST, MAX_SCALE};
use super::{Active, TEST_LOCK, active, rewrite, set_active};

/// The old trees profile as a set, merged: what the one mechanism makes of a profile.
pub(super) fn build(profile: &ModelSwapProfile, models: Vec<Solid>, textures: Vec<Texture>) -> Active {
    let set = Arc::new(from_model_swap("x", profile));
    Active::build(vec![LoadedSet { owner: "mod".into(), order: 0, set, models, added: textures, replaced: Vec::new() }])
}

/// `model` fitted into the local solid `stock`.
fn fit(model: &Solid, stock: &Solid, how: SwapFit) -> anyhow::Result<Solid> {
    let fitted = fit_prepared(&Prepared::new(model)?, stock, how, Frame::Local, &FitLimits::default())?;
    fitted.ok_or_else(|| anyhow::anyhow!("the solid keeps its own model"))
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

/// A solid of leaf cards (the way the game's rows of trees are) whose vertices are `points`.
fn cloud(name: &str, points: &[[f32; 3]]) -> Solid {
    let vertices: Vec<Vertex> = points.iter().map(|&p| vertex(p, [0.0, 0.0])).collect();
    let indices: Vec<u16> = (0..points.len() as u16 - 2).flat_map(|n| [n, n + 1, n + 2]).collect();
    let leaves = GroupSource { effect_id: 6, flags: 0x4008, ..GroupSource::new(vertices, indices) };
    let mut source = SolidSource::new(name, vec![leaves]);
    source.texture_hashes = vec![bstring_hash("STOCKTEX")];
    write(source)
}

/// A pack whose one swap replaces `solid` with the tree model, fitted as `fit` ("height" or "row").
fn pack_for(solid: &str, fit: &str) -> Active {
    let text = format!(
        r#"format = 1
geometry = "TREES/X/GEOMETRY.BIN"
textures = "TREES/X/TEXTURES.BIN"

[[swap]]
solid = "{solid}"
model = "MW12_TREE_L0"
fit = "{fit}"
"#
    );
    let models = vec![tree("MW12_TREE_L0", "MW12_LEAF", 10.0, 3.0, 0)];
    build(&parse_model_swap_profile(&text).unwrap(), models, vec![texture("MW12_LEAF")])
}

/// A solid with only the bounds `low` to `high`: a quad across them.
fn bounds_box(name: &str, low: [f32; 3], high: [f32; 3]) -> Solid {
    let corners = [low, [high[0], low[1], low[2]], high, [low[0], high[1], high[2]]];
    let trunk = GroupSource { effect_id: 0, flags: 0x4000, ..GroupSource::new(quad(corners), vec![0, 1, 2, 0, 2, 3]) };
    let mut source = SolidSource::new(name, vec![trunk]);
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
fn a_local_tree_stands_on_the_origin_and_reaches_the_top_of_the_solid_it_replaces() {
    let stock = bounds_box("XT_BOX", [-5.0, -5.0, 5.0], [5.0, 5.0, 25.0]);
    let fitted = fit(&tree("MODEL", "LEAF", 10.0, 3.0, 0), &stock, SwapFit::Height).unwrap();
    assert!(lowest(&fitted).abs() < 0.1, "the origin is the ground, whatever the old floor: {}", lowest(&fitted));
    assert!((highest(&fitted) - 25.0).abs() < 0.1);
}

#[test]
fn a_replacement_is_never_wider_than_the_footprint_it_replaces_by_more_than_the_spread() {
    // A stock tree 8 metres wide and 20 tall; the model is 6 wide and 10 tall, so the height asks for twice its size (12 metres
    // wide). It is squeezed to 8 x 1.25 = 10 metres.
    let stock = bounds_box("XT_THIN", [-4.0, -4.0, 0.0], [4.0, 4.0, 20.0]);
    let fitted = fit(&tree("MODEL", "LEAF", 10.0, 3.0, 0), &stock, SwapFit::Height).unwrap();
    let width = fitted.bounds_max[0] - fitted.bounds_min[0];
    assert!(width <= 10.1, "held to the spread: {width}");
    assert!((highest(&fitted) - 20.0).abs() < 0.1, "squeezed, not shortened");
}

#[test]
fn a_replacement_that_cannot_fit_leaves_the_solid_its_own_model() {
    let narrow = bounds_box("XT_NARROW", [-0.5, -0.5, 0.0], [0.5, 0.5, 20.0]);
    let model = Prepared::new(&tree("MODEL", "LEAF", 10.0, 3.0, 0)).unwrap();
    let limits = FitLimits::default();
    assert!(fit_prepared(&model, &narrow, SwapFit::Height, Frame::Local, &limits).unwrap().is_none());
    let loose = FitLimits { min_height: 0.0, ..limits };
    let tiny = fit_prepared(&model, &narrow, SwapFit::Height, Frame::Local, &loose).unwrap().unwrap();
    assert!(highest(&tiny) < 10.0, "with no least height it is made as small as the footprint asks");
}

#[test]
fn a_row_is_one_copy_for_each_stretch_of_the_old_geometry() {
    let model = tree("MODEL", "LEAF", 10.0, 3.0, 0);
    let points: Vec<[f32; 3]> = (0..=24)
        .flat_map(|n| (0..2).map(move |side| [n as f32 * 2.5, side as f32 * 6.0, 20.0 * side as f32 + 0.6 * n as f32]))
        .collect();
    let stock = cloud("XT_ROW", &points);
    let fitted = fit(&model, &stock, SwapFit::Row).unwrap();
    let copies = fitted.groups.len() / 2;
    assert!((3..=ROW_COPIES_AT_MOST).contains(&copies), "{copies} copies of a 60 metre row");
    for vertex in &fitted.vertices {
        assert!(vertex.position[0] > -6.0 && vertex.position[0] < 66.0, "inside the row: {:?}", vertex.position);
    }
}

#[test]
fn a_row_that_climbs_has_every_copy_on_its_own_floor() {
    let points: Vec<[f32; 3]> = (0..=40)
        .flat_map(|n| {
            let x = n as f32 * 2.5;
            [[x, 0.0, 0.3 * x], [x, 4.0, 0.3 * x + 12.0]]
        })
        .collect();
    let stock = cloud("XT_SLOPE_DEINST1_T1", &points);
    let pack = pack_for("XT_SLOPE", "row");
    let mut data = section(vec![stock]);
    pack.apply(&mut data);
    let fitted = &data.solids[0];
    assert_ne!(fitted.vertices.len(), points.len(), "it was replaced");
    let (low, high) = (lowest(fitted), highest(fitted));
    assert!(low < 3.0, "the first copy stands low: {low}");
    assert!(high > 30.0, "the last copy stands on the slope, 30 metres up: {high}");
    let feet: Vec<f32> = fitted
        .groups
        .iter()
        .filter(|group| group.effect_id == 0)
        .map(|group| {
            let start = group.base_vertex as usize;
            let own = &fitted.vertices[start..start + group.num_vertices as usize];
            own.iter().map(|v| v.position[2]).fold(f32::MAX, f32::min)
        })
        .collect();
    assert!(feet.len() >= 3, "{feet:?}");
    assert!(feet.windows(2).all(|pair| pair[1] > pair[0]), "each copy's foot is higher than the one before: {feet:?}");
}

#[test]
fn a_height_fit_is_kept_in_range() {
    let mut model = tree("MODEL", "LEAF", 1.0, 3.0, 0);
    model.bounds_max[2] = 1.0;
    let tall = bounds_box("XT_TALL", [-40.0, -40.0, 0.0], [40.0, 40.0, 100.0]);
    let fitted = fit(&model, &tall, SwapFit::Height).unwrap();
    assert!(highest(&fitted) <= MAX_SCALE + 0.1, "no more than {MAX_SCALE} times its size: {}", highest(&fitted));
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

fn pack() -> Active {
    let models = vec![tree("MW12_TREE_L0", "MW12_LEAF", 10.0, 3.0, 0)];
    build(&profile("XT_STOCK", "MW12_TREE_L0"), models, vec![texture("MW12_LEAF"), texture("UNUSED")])
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
fn a_row_in_world_space_follows_a_diagonal_and_a_bend() {
    // A strip that runs along the diagonal and then turns to run along y: a straight line of copies could not follow it.
    let mut points: Vec<[f32; 3]> = Vec::new();
    for n in 0..=56 {
        let at = n as f32 * 1.5;
        points.push([at, at, 0.0]);
        points.push([at + 5.0, at - 5.0, 18.0]);
    }
    for n in 1..=56 {
        let at = 84.0 + n as f32 * 1.5;
        points.push([84.0, at, 0.0]);
        points.push([89.0, at, 18.0]);
    }
    let copy = cloud("XT_ROW_DEINST1_T1", &points);
    let pack = pack_for("XT_ROW", "row");
    let mut data = section(vec![copy]);
    pack.apply(&mut data);
    let fitted = &data.solids[0];
    let (low, high) = (fitted.bounds_min, fitted.bounds_max);
    assert!(high[0] > 85.0 && low[0] < 5.0, "along the diagonal: {low:?} {high:?}");
    assert!(high[1] > 150.0, "and up the second leg: {high:?}");
}

#[test]
fn a_world_single_tree_is_placed_at_the_middle_of_its_footprint() {
    let copy = world_tree("XT_STOCK_DEINST3_T7", [10.0, 20.0], 20.0);
    let mut data = section(vec![copy.clone()]);
    pack().apply(&mut data);
    let fitted = &data.solids[0];
    let (low, high) = (fitted.bounds_min, fitted.bounds_max);
    let middle = [0.5 * (low[0] + high[0]), 0.5 * (low[1] + high[1])];
    assert!((middle[0] - 10.0).abs() < 1.5 && (middle[1] - 20.0).abs() < 1.5, "{low:?} {high:?}");
    assert!((lowest(fitted) - lowest(&copy)).abs() < 0.1);
}

#[test]
fn a_world_copy_that_cannot_be_fitted_is_left_as_it_is() {
    let mut copy = world_tree("XT_STOCK_DEINST3_T7", [10.0, 20.0], 20.0);
    for vertex in &mut copy.vertices {
        vertex.position[0] = 10.0 + (vertex.position[0] - 10.0) * 0.05;
        vertex.position[1] = 20.0 + (vertex.position[1] - 20.0) * 0.05;
    }
    copy.bounds_min = [9.0, 19.0, copy.bounds_min[2]];
    copy.bounds_max = [11.0, 21.0, copy.bounds_max[2]];
    let before = copy.clone();
    let mut data = section(vec![copy]);
    pack().apply(&mut data);
    assert_eq!(data.solids[0], before, "the game's own tree stays");
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
    let nobody = build(&profile("XT_STOCK", "NO_SUCH_MODEL"), models.clone(), vec![texture("MW12_LEAF")]);
    assert_eq!(nobody.swaps(), 0);
    let bare = build(&profile("XT_STOCK", "MW12_TREE_L0"), models, vec![texture("OTHER")]);
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

#[test]
fn a_world_copy_of_a_mirrored_instance_is_wound_the_other_way() {
    use blackbox_scenery::{SceneryInfo, SceneryInstance, ScenerySection};
    let copy = world_tree("XT_STOCK_DEINST3_T7", [0.0, 0.0], 20.0);
    let plain = world_tree("XT_STOCK_DEINST4_T7", [30.0, 0.0], 20.0);
    let instance = |info_index: i16, exclude_flags: u32| SceneryInstance {
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        exclude_flags,
        preculler_index: 0,
        lighting_context: 0,
        position: [0.0; 3],
        rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        info_index,
    };
    let info = |name: &str, hash: u32| SceneryInfo {
        name: name.to_owned(),
        solid_keys: vec![hash],
        radius: 1.0,
        hierarchy_hash: 0,
    };
    let scenery = ScenerySection {
        section_number: 1,
        infos: vec![info("A", copy.name_hash), info("B", plain.name_hash)],
        instances: vec![instance(0, 0x0040_0000), instance(1, 0)],
        override_hooks: Vec::new(),
    };
    let mut data = section(vec![copy, plain]);
    data.scenery = vec![scenery];
    pack().apply(&mut data);
    assert_eq!(data.solids[0].indices[..3], [0, 2, 1], "the mirror is baked in, as the game's own copy has it");
    assert_eq!(data.solids[1].indices[..3], [0, 1, 2], "an unmirrored copy is wound as the model is");
}
