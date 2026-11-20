use blackbox_hash::bstring_hash;
use glam::{Mat4, Vec3};

use super::*;
use crate::car::Placement;
use crate::car::exhaust::{pipe_direction, pipe_frames};
use crate::car::mods::components::parts::add_solid;

fn solid(name: &str, markers: Vec<PositionMarker>) -> Solid {
    Solid {
        secondary_uvs: None,
        tangents: None,
        name: name.into(),
        name_hash: bstring_hash(name),
        version: 0,
        flags: 0,
        bounds_min: [0.0; 3],
        bounds_max: [0.0; 3],
        transform: [0.0; 16],
        num_polys: 0,
        density: 0.0,
        texture_hashes: Vec::new(),
        light_material_hashes: Vec::new(),
        markers,
        vertex_buffers: Vec::new(),
        vertices: Vec::new(),
        indices: Vec::new(),
        groups: Vec::new(),
        skin: None,
    }
}

fn pipe(name: &str, position: [f32; 3], direction: [f32; 3]) -> PartMarker {
    PartMarker { name: name.to_owned(), position, direction }
}

fn binding(solid: &str, matrix: Option<[f32; 12]>, markers: Vec<PartMarker>) -> Binding {
    Binding {
        solid: solid.to_owned(),
        pack: Some("PARTS/R6/X".to_owned()),
        role: Role::Body,
        matrix,
        base: false,
        markers,
    }
}

fn translation(x: f32, y: f32, z: f32) -> [f32; 12] {
    [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z]
}

fn body_placement(solid: u32) -> Placement {
    Placement { solid, transform: Mat4::IDENTITY, left_brake: false, slot: 0, corner: None }
}

/// The stock bike: a body with its pipe at the bottom (and a headlight marker that is not a pipe), and its placement.
fn stock() -> (HashMap<u32, Solid>, Vec<Placement>) {
    let at_the_bottom = marker(&pipe("EXHAUST", [-0.7, 0.0, 0.25], [-1.0, 0.0, 0.0]));
    let headlight = PositionMarker { name_hash: bstring_hash("LEFT_HEADLIGHT"), ..at_the_bottom };
    let body = solid("BODY_A", vec![at_the_bottom, headlight]);
    let placement = body_placement(body.name_hash);
    (HashMap::from([(body.name_hash, body)]), vec![placement])
}

/// Fit `bindings` (id, binding) on the stock bike the way `parts::place` does for one level: strip the stock pipes when a part sets
/// its own, then add each part's solid and place it. The frames of the pipes the flames would start at.
fn fitted(bindings: &[(&str, Binding)]) -> Vec<Mat4> {
    let (mut solids, mut placements) = stock();
    let pack: HashMap<u32, Solid> = bindings
        .iter()
        .map(|(_, binding)| bstring_hash(&format!("{}_A", binding.solid)))
        .map(|hash| (hash, solid("PACK", Vec::new())))
        .collect();
    if bindings.iter().any(|(_, binding)| sets_pipes(binding)) {
        strip_pipes(&mut solids);
    }
    for (id, binding) in bindings {
        let key = add_solid(&mut solids, &pack, id, binding, 0).expect("the pack has the solid");
        placements.push(body_placement(key));
    }
    pipe_frames(&placements, &solids)
}

fn at(frame: &Mat4) -> Vec3 {
    frame.w_axis.truncate()
}

#[test]
fn the_marker_of_a_pipe_has_its_flame_on_z_and_its_place_in_the_last_row() {
    let back = marker(&pipe("RIGHT_EXHAUST", [-0.9, -0.1, 0.8], [-1.0, 0.0, 0.0]));
    assert_eq!(back.name_hash, bstring_hash("RIGHT_EXHAUST"));
    assert_eq!(&back.matrix[8..11], &[-1.0, 0.0, 0.0], "the z row is the way the flame shoots");
    assert_eq!(&back.matrix[12..16], &[-0.9, -0.1, 0.8, 1.0]);
    assert_eq!(&back.matrix[0..3], &[0.0, 0.0, 1.0], "x points up");
    let up = marker(&pipe("EXHAUST", [0.0; 3], [0.0, 0.0, 1.0]));
    assert_eq!(&up.matrix[8..11], &[0.0, 0.0, 1.0]);
    assert_eq!(&up.matrix[0..3], &[1.0, 0.0, 0.0], "x points forward for a pipe that shoots up");
    let slanted = marker(&pipe("EXHAUST", [0.0; 3], [-2.0, 0.0, 2.0]));
    let z = Vec3::from_slice(&slanted.matrix[8..11]);
    assert!((z.length() - 1.0).abs() < 1e-6 && z.x < 0.0 && z.z > 0.0, "a direction is made a unit vector: {z:?}");
    // An axis of no length is not a pipe pointing nowhere: it shoots backwards.
    let none = marker(&pipe("EXHAUST", [0.0; 3], [0.0; 3]));
    assert_eq!(&none.matrix[8..11], &[-1.0, 0.0, 0.0]);
}

#[test]
fn only_a_body_part_that_names_a_tail_pipe_sets_the_pipes() {
    let one = vec![pipe("LEFT_EXHAUST", [0.0; 3], [-1.0, 0.0, 0.0])];
    assert!(sets_pipes(&binding("M", None, one.clone())));
    assert!(!sets_pipes(&binding("M", None, Vec::new())), "a part with no markers leaves the stock pipes");
    assert!(!sets_pipes(&binding("M", None, vec![pipe("LEFT_HEADLIGHT", [0.0; 3], [1.0, 0.0, 0.0])])));
    let steered = Binding { role: Role::Steered, ..binding("M", None, one) };
    assert!(!sets_pipes(&steered) && pipe_markers(&steered).is_empty(), "the pipes are in the body's frame");
}

#[test]
fn the_stock_pipes_stay_without_a_part_that_says_where_its_own_are() {
    let frames = fitted(&[("exhaust.stock", binding("S", None, Vec::new()))]);
    assert_eq!(frames.len(), 1);
    assert!((at(&frames[0]) - Vec3::new(-0.7, 0.0, 0.25)).length() < 1e-6);
    // An exhaust imported before the markers were written does not take the pipes away either.
    let old = fitted(&[("exhaust.muffler.race", binding("M", Some(translation(0.0, 0.0, 0.6)), Vec::new()))]);
    assert_eq!(old.len(), 1);
    assert!((at(&old[0]) - Vec3::new(-0.7, 0.0, 0.25)).length() < 1e-6);
}

#[test]
fn a_fitted_exhaust_moves_the_pipe_to_its_own_outlet_and_takes_the_stock_one_away() {
    // A top-mounted silencer: its outlet is 80 cm above the stock pipe's, 20 cm further back.
    let top = binding(
        "TOP",
        Some(translation(0.0, 0.0, 0.8)),
        vec![pipe("RIGHT_EXHAUST", [-0.9, -0.1, 1.05], [-0.97, 0.0, 0.25])],
    );
    let frames = fitted(&[("exhaust.muffler.top", top)]);
    assert_eq!(frames.len(), 1, "the stock pipe is gone: one pipe, the fitted one");
    assert!((at(&frames[0]) - Vec3::new(-0.9, -0.1, 1.05)).length() < 1e-6, "{:?}", at(&frames[0]));
    let moved = at(&frames[0]).z - 0.25;
    assert!(moved > 0.79, "the emitter is {moved} m higher than the stock one's");
    let direction = pipe_direction(&frames[0]);
    assert!((direction - Vec3::new(-0.97, 0.0, 0.25).normalize()).length() < 1e-5, "{direction:?}");
    // The body keeps its other markers.
    let (mut solids, _) = stock();
    strip_pipes(&mut solids);
    let body = solids.values().next().unwrap();
    assert_eq!(body.markers.len(), 1);
    assert_eq!(body.markers[0].name_hash, bstring_hash("LEFT_HEADLIGHT"));
}

#[test]
fn a_twin_silencer_is_two_pipes_one_for_each_can() {
    let can = |id: &str, y: f32| {
        let marker = vec![pipe("EXHAUST", [-0.8, y, 0.95], [-1.0, 0.0, 0.0])];
        (id.to_owned(), binding("TWIN", Some(translation(0.0, y, 0.7)), marker))
    };
    let (first, second) = (can("exhaust.muffler.t", -0.12), can("exhaust.muffler2.t", 0.12));
    let both = fitted(&[(first.0.as_str(), first.1), (second.0.as_str(), second.1)]);
    assert_eq!(both.len(), 2, "both cans flame");
    let mut ys: Vec<f32> = both.iter().map(|frame| at(frame).y).collect();
    ys.sort_by(f32::total_cmp);
    assert_eq!(ys, [-0.12, 0.12]);
    assert!(both.iter().all(|frame| (at(frame).z - 0.95).abs() < 1e-6));
}
