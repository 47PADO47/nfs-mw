//! Traffic lane numbering: the n-th traffic lane from the centre, or from the curb.

use crate::{RoadProfile, zone};

/// The traffic lanes that run in the travel direction (right of the middle), centre first.
pub fn forward_traffic_lanes(profile: &RoadProfile) -> Vec<usize> {
    profile.lanes_of(zone::TRAFFIC, true)
}

/// The lane to continue in on a new profile: the `nth` traffic lane from the centre, or from the curb
/// when `from_curb`. Falls back to the nearest traffic lane when `nth` is out of range, and to the
/// middle zone when there is none.
pub fn pick_lane(profile: &RoadProfile, nth: usize, from_curb: bool) -> usize {
    let lanes = forward_traffic_lanes(profile);
    let Some(last) = lanes.len().checked_sub(1) else {
        return usize::from(profile.middle).min(profile.zones.len().saturating_sub(1));
    };
    match from_curb {
        true => lanes[last - nth.min(last)],
        false => lanes[nth.min(last)],
    }
}

/// Position of `lane` among the forward traffic lanes counted from the centre, if it is one.
pub fn nth_from_centre(profile: &RoadProfile, lane: usize) -> Option<usize> {
    forward_traffic_lanes(profile).iter().position(|&l| l == lane)
}
