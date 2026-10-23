use glam::Vec3;

use super::synth::*;
use crate::{SegmentFilter, SegmentIndex, closest_segment};

fn pick(y: f32) -> u16 {
    let net = stacked_roads();
    let index = SegmentIndex::build(&net);
    let point = Vec3::new(1.0, y, 50.0);
    closest_segment(&net, &index, point, Vec3::Z, 1.0, SegmentFilter::default()).expect("a road").segment
}

#[test]
fn a_car_on_the_lower_road_snaps_to_it() {
    assert_eq!(pick(0.5), 0);
}

#[test]
fn a_car_on_the_bridge_snaps_to_it() {
    assert_eq!(pick(10.5), 1);
}

#[test]
fn a_small_height_difference_costs_nothing() {
    // A car bouncing 2.5 m above the lower road is still on it.
    assert_eq!(pick(2.5), 0);
}
