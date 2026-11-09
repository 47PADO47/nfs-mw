//! Keeping copies off the objects next to them, on authored numbers: no game data.

use std::f32::consts::FRAC_1_SQRT_2;

use blackbox_modding::content::{FitLimits, SwapFit};

use super::super::fit::{Frame, Prepared, fit_clear};
use super::super::tests::{cloud, tree};
use super::*;

const BUSHES: FitLimits = FitLimits { spread: 1.5, squeeze: 0.5, min_height: 0.5, clearance: true, settle: false };

/// A model 4 metres wide and deep and 2 tall, its foot on the origin.
const MODEL: Measure =
    Measure { width: 4.0, height: 2.0, vertices: 8, low: [-2.0, -2.0, 0.0], high: [2.0, 2.0, 2.0], crown_low: 0.0 };

/// A copy at `x`, 1.5 times as wide as the stretch (4 metres) it replaces.
fn copy_at(x: f32) -> Placement {
    Placement { across: 1.5, up: 1.5, yaw: 0.0, offset: [x, 0.0, 0.0], base: 1.0 }
}

/// A barrier across x from `from` to `to`, 10 metres long in y and a metre high.
fn barrier(from: f32, to: f32) -> Wall {
    Wall { quad: plan_of([from, -5.0], [to, 5.0]), low: 0.0, high: 1.0 }
}

fn clear_world(walls: Vec<Wall>, placements: &mut Vec<Placement>) -> Tally {
    let near = Neighbours::from_parts(walls, &[]);
    near.clear(None, ([0.0; 3], [20.0, 1.0, 3.0]), &MODEL, placements, &BUSHES)
}

/// The farthest x the copy reaches.
fn reach(at: &Placement) -> f32 {
    at.offset[0] + 2.0 * at.across
}

#[test]
fn a_bush_beside_a_barrier_is_made_narrower_until_it_clears_it() {
    // At 1.5 the copy reaches x = 13, into the barrier that starts at 12.5.
    let mut placements = vec![copy_at(10.0)];
    let tally = clear_world(vec![barrier(12.5, 12.8)], &mut placements);
    assert_eq!(tally, Tally { copies: 1, shrunk: 1, held: 0, skipped: 0 });
    let at = placements[0];
    assert!(reach(&at) < 12.5, "it clears the barrier: {at:?}");
    assert!(
        at.across >= at.base && at.across < 1.5,
        "it is narrower than placed and no narrower than the stretch it replaces: {at:?}"
    );
    assert_eq!(at.up, 1.5, "its height stays when the squeeze limit allows it");
}

#[test]
fn a_bush_with_nothing_beside_it_is_left_as_placed() {
    let mut placements = vec![copy_at(10.0)];
    let tally = clear_world(vec![barrier(40.0, 40.3)], &mut placements);
    assert_eq!(
        tally,
        Tally { copies: 0, shrunk: 0, held: 0, skipped: 0 },
        "a barrier out of reach is not even looked at"
    );
    assert_eq!(placements, vec![copy_at(10.0)]);
}

#[test]
fn an_object_at_another_height_does_not_stop_a_bush() {
    let mut overhead = barrier(12.5, 12.8);
    overhead.low = 6.0;
    overhead.high = 7.0;
    let mut placements = vec![copy_at(10.0)];
    let tally = clear_world(vec![overhead], &mut placements);
    assert_eq!((tally.shrunk, tally.skipped), (0, 0), "a deck over the bush is not a barrier beside it");
    assert_eq!(placements, vec![copy_at(10.0)]);
}

#[test]
fn a_bush_never_gets_narrower_than_the_stretch_it_replaces() {
    // The barrier is against the old footprint (x = 12): even the base width touches it, so the copy keeps that width.
    let mut placements = vec![copy_at(10.0)];
    let tally = clear_world(vec![barrier(11.5, 11.8)], &mut placements);
    assert_eq!(tally, Tally { copies: 1, shrunk: 1, held: 1, skipped: 0 });
    assert!((placements[0].across - placements[0].base).abs() < 1e-5, "{:?}", placements[0]);
}

#[test]
fn a_bush_rooted_inside_an_object_is_left_out() {
    let mut placements = vec![copy_at(10.0), copy_at(14.0)];
    let tally = clear_world(vec![barrier(9.0, 11.0)], &mut placements);
    assert_eq!((tally.copies, tally.skipped), (2, 1));
    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].offset[0], 14.0, "the other copy stays");
}

#[test]
fn the_height_follows_when_a_narrow_copy_would_be_too_thin() {
    // Squeezed to a third of its placed width the copy would be thinner than half its height: the height comes down with it.
    let at = Placement { across: 1.5, up: 1.5, yaw: 0.0, offset: [0.0; 3], base: 0.5 };
    let narrow = scaled(&at, 1.0 / 3.0, &BUSHES);
    assert!((narrow.across - 0.5).abs() < 1e-5 && (narrow.up - 1.0).abs() < 1e-5, "across / squeeze: {narrow:?}");
    // And never shorter than min_height of the height wanted: the smallest width is held there.
    let short = Placement { base: 0.1, ..at };
    assert!((smallest_scale(&short, &BUSHES) - 0.25).abs() < 1e-5, "{}", smallest_scale(&short, &BUSHES));
}

#[test]
fn a_copy_in_a_turned_instance_is_tested_where_the_scenery_puts_it() {
    // A model 2 wide in x and 6 deep in y, placed by an instance turned a quarter: its depth runs along the world x axis.
    let long =
        Measure { width: 6.0, height: 2.0, vertices: 8, low: [-1.0, -3.0, 0.0], high: [1.0, 3.0, 2.0], crown_low: 0.0 };
    let turned: Matrix = [[0.0, 1.0, 0.0, 0.0], [-1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [100.0, 200.0, 0.0, 1.0]];
    let (low, high) = ([97.0, 197.0, 0.0], [103.0, 203.0, 3.0]);
    let near = Neighbours::from_parts(
        vec![Wall { quad: plan_of([102.2, 190.0], [102.6, 210.0]), low: 0.0, high: 1.0 }],
        &[(7, turned, low, high)],
    );
    let at = Placement { across: 1.0, up: 1.0, yaw: 0.0, offset: [0.0; 3], base: 0.5 };
    let mut placements = vec![at];
    let tally = near.clear(Some(7), ([-1.0, -3.0, 0.0], [1.0, 3.0, 2.0]), &long, &mut placements, &BUSHES);
    assert_eq!(tally.shrunk, 1, "{tally:?}");
    assert!(3.0 * placements[0].across < 2.2, "the depth, turned onto x, stops short of the wall: {:?}", placements[0]);
    // A solid the section does not place has no place to be tested at.
    let mut other = vec![at];
    assert_eq!(near.clear(Some(8), ([0.0; 3], [1.0; 3]), &long, &mut other, &BUSHES), Tally::default());
    assert_eq!(other, vec![at]);
}

#[test]
fn only_walls_rails_barriers_and_buildings_are_objects() {
    for name in ["XW_GUARDRAILA_1B_00", "xw_barrconcb_1b_00", "XB_HAPARTMENT_1B_00"] {
        assert!(is_obstacle(name), "{name}");
    }
    for name in ["XT_BUSHGROUP_1B_00", "XW_HEDGESTADIUM_1B_00", "XO_STREETLIGHT_1B", "XS_StopSignB", "TRK_UNIT1", "XW"]
    {
        assert!(!is_obstacle(name), "{name}");
    }
}

#[test]
fn the_oriented_box_of_a_known_solid_replaces_the_instance_box_only_when_they_agree() {
    let turned: Matrix = [
        [FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0, 0.0],
        [-FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [10.0, 10.0, 0.0, 1.0],
    ];
    // A rail 6 long and 0.4 thick turned 45 degrees: its box is 4.5 square, its oriented plan a thin diagonal.
    let rail = Some(([-3.0, -0.2, 0.0], [3.0, 0.2, 1.0]));
    let wall = wall_of(&turned, rail, [7.5, 7.5, 0.0], [12.5, 12.5, 1.0]);
    let tight = overlap(&wall.quad, &plan_of([7.5, 11.5], [8.5, 12.5]));
    assert!(!tight, "the corner of the instance's box is empty: the rail does not reach it");
    // A solid whose bounds land outside the instance's box is not the one placed (a stale registry entry): the box is used.
    let far = wall_of(&turned, Some(([-30.0, -0.2, 0.0], [30.0, 0.2, 1.0])), [7.5, 7.5, 0.0], [12.5, 12.5, 1.0]);
    assert!(overlap(&far.quad, &plan_of([7.5, 11.5], [8.5, 12.5])));
}

#[test]
fn a_row_of_copies_keeps_off_a_barrier_standing_at_its_start() {
    // A stretch of low growth along x from 0 to 8, a model 4 wide and a metre tall: the first copy (it is not turned) is as wide as the
    // stretch allows, 1 metre either side of its cell, and reaches half a metre before the stretch begins.
    let points: Vec<[f32; 3]> =
        (0..=8).flat_map(|x| [[x as f32, 0.0, 0.0], [x as f32, 0.0, 1.0], [x as f32, 1.0, 0.5]]).collect();
    let stock = cloud("XT_BUSHGROUP_1B_00_DEINST1_T1", &points);
    let model = Prepared::new(&tree("MW12_BUSH_L0", "LEAF", 1.0, 2.0, 0)).unwrap();
    let fit = |limits: &FitLimits, near: &Neighbours| {
        let (solid, tally) =
            fit_clear(&model, &stock, SwapFit::Row, Frame::World { mirrored: false }, limits, near).unwrap();
        let lowest = solid.unwrap().vertices.iter().map(|v| v.position[0]).fold(f32::MAX, f32::min);
        (lowest, tally)
    };
    let (open, _) = fit(&BUSHES, &Neighbours::none());
    assert!(open < -0.45, "without the barrier the first copy reaches over the start: {open}");
    let barrier = Wall { quad: plan_of([-0.8, -1.0], [-0.4, 2.0]), low: 0.0, high: 1.0 };
    let near = Neighbours::from_parts(vec![barrier], &[]);
    let (kept, tally) = fit(&BUSHES, &near);
    assert!(kept > -0.41, "with it the copy stops short of it: {kept}");
    assert!(tally.shrunk >= 1 && tally.skipped == 0, "{tally:?}");
    let off = FitLimits { clearance: false, ..BUSHES };
    assert_eq!(fit(&off, &near).0, open, "a set that does not ask for clearance is fitted as before");
}

#[test]
fn the_top_of_an_object_holds_up_a_foot_that_is_on_it() {
    let near = Neighbours::from_parts(vec![barrier(0.0, 4.0)], &[]);
    assert!(near.supports([2.0, 0.0], 1.0));
    assert!(near.supports([4.3, 0.0], 1.2), "within the slack of its edge and its top");
    assert!(!near.supports([6.0, 0.0], 1.0), "past the edge");
    assert!(!near.supports([2.0, 0.0], 2.0), "a metre over its top");
    assert!(!near.supports([2.0, 0.0], 0.0), "at its foot: that is the ground's, not the object's");
}
