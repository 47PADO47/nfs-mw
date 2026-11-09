//! Settling copies on authored ground, roads and walls: no game data.

use blackbox_modding::content::{FitLimits, SwapFit};

use super::super::audit::judge;
use super::super::clear::Wall;
use super::super::fit::{Frame, Prepared, fit_site};
use super::super::ground::Terrain;
use super::super::roads::{Line, Roads};
use super::super::tests::{cloud, tree};
use super::*;

/// A tree 20 metres tall and 10 wide, its crown beginning 5 metres up.
const MODEL: Measure = Measure {
    width: 10.0,
    height: 20.0,
    vertices: 100,
    low: [-5.0, -5.0, 0.0],
    high: [5.0, 5.0, 20.0],
    crown_low: 5.0,
};
/// What a row's copies are scaled up to at most.
const CAP: f32 = 1.5;

fn limits() -> FitLimits {
    FitLimits { settle: true, ..FitLimits::default() }
}

fn copy(x: f32, y: f32, z: f32) -> Placement {
    Placement { across: 1.0, up: 1.0, yaw: 0.0, offset: [x, y, z], base: 1.0 }
}

/// Flat ground at `z` under x and y from -50 to 150.
fn flat(z: f32) -> Terrain {
    Terrain::from_triangles(vec![
        [[-50.0, -50.0, z], [150.0, -50.0, z], [150.0, 150.0, z]],
        [[-50.0, -50.0, z], [150.0, 150.0, z], [-50.0, 150.0, z]],
    ])
    .unwrap()
}

/// A road along x at y = 0 from 0 to 100 with the half width and height given.
fn road(half: f32, level: f32) -> Line {
    Line { points: vec![([0.0, 0.0], level, half), ([50.0, 0.0], level, half), ([100.0, 0.0], level, half)] }
}

fn wall_top(from: [f32; 2], to: [f32; 2], high: f32) -> Wall {
    Wall { quad: [[from[0], from[1]], [to[0], from[1]], [to[0], to[1]], [from[0], to[1]]], low: 0.0, high }
}

/// `placements` settled at a site of this ground, these roads and these walls.
fn settled(
    model: &Measure,
    placements: &mut Vec<Placement>,
    terrain: Option<&Terrain>,
    roads: Option<&Roads>,
    walls: Vec<Wall>,
) -> Settled {
    let near = Neighbours::from_parts(walls, &[]);
    settle(model, placements, CAP, &Site { terrain, roads, near: &near }, &limits())
}

#[test]
fn a_crown_that_hangs_over_the_ground_comes_down_and_grows_to_keep_its_top() {
    let ground = flat(0.0);
    let mut placements = vec![copy(5.0, 5.0, 7.4)];
    let tally = settled(&MODEL, &mut placements, Some(&ground), None, Vec::new());
    assert_eq!(placements[0].offset, [5.0, 5.0, 0.0], "the foot is on the ground");
    assert!((placements[0].up - 27.4 / 20.0).abs() < 1e-4, "the top stays at 27.4: {:?}", placements[0]);
    assert_eq!((tally.copies, tally.lowered, tally.raised, tally.dropped), (1, 1, 0, 0));
}

#[test]
fn a_lowered_tree_never_grows_past_the_scale_cap() {
    let ground = flat(0.0);
    let mut placements = vec![copy(5.0, 5.0, 11.0)];
    settled(&MODEL, &mut placements, Some(&ground), None, Vec::new());
    assert_eq!(placements[0].up, CAP, "31 metres would be 1.55 times, the cap is 1.5");
    assert_eq!(placements[0].offset[2], 0.0);
}

#[test]
fn a_tree_that_sank_comes_up_and_loses_a_little_height() {
    let ground = flat(0.0);
    let mut placements = vec![copy(5.0, 5.0, -1.0)];
    let tally = settled(&MODEL, &mut placements, Some(&ground), None, Vec::new());
    assert_eq!(placements[0].offset[2], 0.0);
    assert!((placements[0].up - 19.0 / 20.0).abs() < 1e-4, "its top stays: {:?}", placements[0]);
    assert_eq!((tally.raised, tally.lowered), (1, 0));
}

#[test]
fn a_raised_tree_keeps_at_least_six_tenths_of_its_height() {
    let ground = flat(0.0);
    let small = Measure { height: 4.0, high: [5.0, 5.0, 4.0], ..MODEL };
    let mut placements = vec![copy(5.0, 5.0, -3.0)];
    settled(&small, &mut placements, Some(&ground), None, Vec::new());
    assert!(
        (placements[0].up - 0.6).abs() < 1e-4,
        "its top would be at 1 m: a quarter; it keeps 0.6: {:?}",
        placements[0]
    );
}

#[test]
fn a_foot_within_the_tolerance_is_left_as_placed() {
    let ground = flat(0.0);
    let before = copy(5.0, 5.0, 0.25);
    let mut placements = vec![before];
    let tally = settled(&MODEL, &mut placements, Some(&ground), None, Vec::new());
    assert_eq!(placements, vec![before]);
    assert_eq!((tally.lowered, tally.raised, tally.shifted, tally.narrowed, tally.dropped), (0, 0, 0, 0, 0));
}

#[test]
fn no_terrain_or_none_in_the_window_leaves_the_foot_alone() {
    let before = copy(5.0, 5.0, 7.4);
    let mut none = vec![before];
    settled(&MODEL, &mut none, None, None, Vec::new());
    assert_eq!(none, vec![before], "no terrain solids in the section");
    let ground = flat(0.0);
    let high = copy(5.0, 5.0, 13.0);
    let mut far = vec![high];
    settled(&MODEL, &mut far, Some(&ground), None, Vec::new());
    assert_eq!(far, vec![high], "13 m up is more than the window below the foot reaches");
}

#[test]
fn a_crown_that_begins_at_the_top_of_a_wall_stands_on_it() {
    let ground = flat(0.0);
    let on_wall = copy(5.0, 5.0, 3.0);
    let mut held = vec![on_wall];
    settled(&MODEL, &mut held, Some(&ground), None, vec![wall_top([0.0, 0.0], [10.0, 10.0], 3.0)]);
    assert_eq!(held, vec![on_wall], "the wall's top holds it up");
    let mut beside = vec![on_wall];
    settled(&MODEL, &mut beside, Some(&ground), None, vec![wall_top([20.0, 0.0], [30.0, 10.0], 3.0)]);
    assert_eq!(beside[0].offset[2], 0.0, "a wall that is not under it holds nothing up");
    let mut lower = vec![on_wall];
    settled(&MODEL, &mut lower, Some(&ground), None, vec![wall_top([0.0, 0.0], [10.0, 10.0], 1.0)]);
    assert_eq!(lower[0].offset[2], 0.0, "a wall whose top is 2 m under the foot holds nothing up");
}

#[test]
fn a_copy_on_the_carriageway_is_moved_to_the_margin_beside_it() {
    let ground = flat(0.0);
    let roads = Roads::from_lines(&[road(4.0, 0.0)]);
    let mut placements = vec![copy(10.0, 1.0, 0.0)];
    let tally = settled(&MODEL, &mut placements, Some(&ground), Some(&roads), Vec::new());
    let [x, y, _] = placements[0].offset;
    assert!((x - 10.0).abs() < 1e-3 && (y - 5.0).abs() < 1e-3, "5 m from the centre, 1 m outside the edge: {x} {y}");
    assert_eq!((tally.shifted, tally.dropped), (1, 0));
}

#[test]
fn a_moved_copy_is_put_on_the_ground_where_it_stands_now() {
    let slope = Terrain::from_triangles(vec![
        [[-50.0, -50.0, -10.0], [150.0, -50.0, -10.0], [150.0, 50.0, 10.0]],
        [[-50.0, -50.0, -10.0], [150.0, 50.0, 10.0], [-50.0, 50.0, 10.0]],
    ])
    .unwrap();
    let roads = Roads::from_lines(&[road(4.0, 0.0)]);
    let mut placements = vec![copy(10.0, 1.0, 0.2)];
    settled(&MODEL, &mut placements, Some(&slope), Some(&roads), Vec::new());
    assert!((placements[0].offset[1] - 5.0).abs() < 1e-3);
    assert!((placements[0].offset[2] - 1.0).abs() < 1e-3, "the ground at y = 5 is 1 m up: {:?}", placements[0].offset);
}

#[test]
fn a_copy_too_far_inside_the_road_to_move_off_it_is_left_out() {
    let ground = flat(0.0);
    let roads = Roads::from_lines(&[road(6.0, 0.0)]);
    let mut placements = vec![copy(10.0, 0.0, 0.0)];
    let tally = settled(&MODEL, &mut placements, Some(&ground), Some(&roads), Vec::new());
    assert!(placements.is_empty(), "7 m to the margin is more than 5");
    assert_eq!(tally.dropped, 1);
}

#[test]
fn a_copy_that_would_be_moved_onto_another_road_is_left_out() {
    let ground = flat(0.0);
    let across = Line { points: vec![([12.0, -50.0], 0.0, 4.0), ([12.0, 50.0], 0.0, 4.0)] };
    let roads = Roads::from_lines(&[road(4.0, 0.0), across]);
    let mut placements = vec![copy(10.0, 2.0, 0.0)];
    settled(&MODEL, &mut placements, Some(&ground), Some(&roads), Vec::new());
    assert!(placements.is_empty(), "a junction: out of one road is into the other");
}

#[test]
fn a_bridge_over_a_tree_or_a_road_under_it_is_not_the_road_it_stands_on() {
    let ground = flat(0.0);
    let over = Roads::from_lines(&[road(4.0, 10.0)]);
    let before = copy(10.0, 1.0, 0.0);
    let mut placements = vec![before];
    let tally = settled(&MODEL, &mut placements, Some(&ground), Some(&over), Vec::new());
    assert_eq!(placements, vec![before], "the road is 10 m up, over the tree");
    assert_eq!(tally.shifted, 0);
}

#[test]
fn a_crown_that_hangs_low_over_a_road_is_narrowed_to_overhang_it_by_half_a_metre() {
    let ground = flat(0.0);
    let roads = Roads::from_lines(&[road(4.0, 0.0)]);
    let low = Measure { crown_low: 3.0, ..MODEL };
    let mut placements = vec![copy(10.0, 8.0, 0.0)];
    let tally = settled(&low, &mut placements, Some(&ground), Some(&roads), Vec::new());
    // The crown reaches 5 m from a foot 4 m from the edge: 1 m over. Allowed 0.5, less the slack: 4.49 of 5 m.
    assert!((placements[0].across - 0.898).abs() < 1e-4, "{:?}", placements[0]);
    assert_eq!(placements[0].offset, [10.0, 8.0, 0.0], "narrowed where it stands");
    assert_eq!(tally.narrowed, 1);
}

#[test]
fn a_crown_that_clears_the_road_by_four_and_a_half_metres_is_left_alone() {
    let ground = flat(0.0);
    let roads = Roads::from_lines(&[road(4.0, 0.0)]);
    let high = Measure { crown_low: 6.0, ..MODEL };
    let before = copy(10.0, 8.0, 0.0);
    let mut placements = vec![before];
    let tally = settled(&high, &mut placements, Some(&ground), Some(&roads), Vec::new());
    assert_eq!(placements, vec![before]);
    assert_eq!(tally.narrowed, 0);
}

#[test]
fn a_crown_that_would_lose_more_than_half_its_width_takes_its_copy_with_it() {
    let ground = flat(0.0);
    let roads = Roads::from_lines(&[road(4.0, 0.0)]);
    let low = Measure { crown_low: 3.0, ..MODEL };
    let mut placements = vec![copy(10.0, 5.5, 0.0)];
    let tally = settled(&low, &mut placements, Some(&ground), Some(&roads), Vec::new());
    assert!(placements.is_empty(), "1.5 m from the edge, a 5 m crown can keep 2 m: 40 percent");
    assert_eq!(tally.dropped, 1);
}

#[test]
fn a_narrowed_crown_keeps_the_squeeze_limit_and_its_base() {
    let ground = flat(0.0);
    let roads = Roads::from_lines(&[road(4.0, 0.0)]);
    let low = Measure { crown_low: 1.5, ..MODEL };
    let tall = Placement { up: 1.8, ..copy(10.0, 8.0, 0.0) };
    let mut placements = vec![tall];
    settled(&low, &mut placements, Some(&ground), Some(&roads), Vec::new());
    let narrowed = placements[0];
    assert!(narrowed.up <= narrowed.across / limits().squeeze + 1e-5, "{narrowed:?}");
    assert!(narrowed.base <= narrowed.across);
}

#[test]
fn the_audit_flags_what_settling_mends() {
    let ground = flat(0.0);
    let roads = Roads::from_lines(&[road(4.0, 0.0)]);
    let low = Measure { crown_low: 3.0, ..MODEL };
    let near = Neighbours::none();
    let site = Site { terrain: Some(&ground), roads: Some(&roads), near: &near };
    let floating = copy(10.0, 12.0, 7.0);
    let verdict = judge(&low, &floating, &site);
    assert!(verdict.floating && !verdict.sunk && !verdict.on_road, "{verdict:?}");
    assert_eq!(verdict.ground, Some(0.0));
    assert!(judge(&low, &copy(10.0, 12.0, -1.0), &site).sunk);
    assert!(judge(&low, &copy(10.0, 1.0, 0.0), &site).on_road);
    assert!(judge(&low, &copy(10.0, 8.0, 0.0), &site).low_crown);
    for at in [floating, copy(10.0, 12.0, -1.0), copy(10.0, 1.0, 0.0), copy(10.0, 8.0, 0.0)] {
        let mut one = vec![at];
        settle(&low, &mut one, CAP, &site, &limits());
        for mended in &one {
            assert!(!judge(&low, mended, &site).any(), "{at:?} became {mended:?}: {:?}", judge(&low, mended, &site));
        }
    }
}

#[test]
fn a_stretch_whose_crowns_hang_over_the_ground_is_fitted_with_its_trunks_on_it() {
    let points: Vec<[f32; 3]> =
        (0..=12).flat_map(|n| [[n as f32 * 2.5, 0.0, 7.0], [n as f32 * 2.5, 4.0, 20.0]]).collect();
    let stock = cloud("XT_TREELINE_DEINST1_T1", &points);
    let model = Prepared::new(&tree("MW12_TREE_L0", "LEAF", 10.0, 3.0, 0)).unwrap();
    let ground = flat(0.0);
    let near = Neighbours::none();
    let site = Site { terrain: Some(&ground), roads: None, near: &near };
    let low = |limits: &FitLimits| {
        let (solid, _, tally) =
            fit_site(&model, &stock, SwapFit::Row, Frame::World { mirrored: false }, limits, &site).unwrap();
        let solid = solid.unwrap();
        (solid.vertices.iter().map(|v| v.position[2]).fold(f32::MAX, f32::min), tally)
    };
    let (floating, none) = low(&FitLimits::default());
    assert!(floating > 6.5, "placed on the floor of the old crowns: {floating}");
    assert_eq!(none.copies, 0, "a set that does not ask to settle is not settled");
    let (settled, tally) = low(&limits());
    assert!(settled.abs() < 0.1, "settled: the trunks reach the ground: {settled}");
    assert!(tally.lowered >= 3, "{tally:?}");
}
