//! Matching real traffic-light props to the approaches of a [`SignalController`](crate::SignalController).
//!
//! The controller knows where each approach stops and which way it drives. The world scenery holds the lamp posts
//! and mast arms. This module is engine-generic: the caller passes the candidate poses and says which of them are
//! mast arms, and gets back one optional candidate per approach. Pure functions: the same input always gives the
//! same matches.
//!
//! Rules, all in the horizontal plane:
//! - Reach: the prop stands within [`MatchParams::max_distance`] of the stop point or of the junction corner (the
//!   point [`STOP_LINE_DISTANCE`] metres past the stop point along the heading; exact for a straight approach).
//! - Facing: the prop faces back along the approach, toward the incoming traffic, within
//!   [`MatchParams::facing_tolerance`] degrees.
//! - Side: the prop stands right of the lane (right of the heading, as seen by the driver), or it is a far-side
//!   prop: beyond the junction corner, across the junction from the approach.
//! - Sharing: a prop serves one approach. Only a mast arm may serve several approaches, and only approaches of
//!   one junction whose headings lie within [`MatchParams::group_tolerance`] degrees of each other.
//! - Choice: the valid pairs are taken best score first (the reach), ties by approach index then prop index.

use glam::Vec3;

use crate::{Approach, STOP_LINE_DISTANCE, right_of};

/// A prop that might be a traffic light, as the caller found it in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropPose {
    /// World position, metres. Only the horizontal plane is matched.
    pub position: Vec3,
    /// The direction the prop faces: the way its lamp heads. A lamp that serves traffic faces back along the
    /// approach, so this is roughly the opposite of the approach heading.
    pub forward: Vec3,
    /// Caller-defined prop kind. The matcher does not read it; the caller gets it back by candidate index.
    pub kind: u32,
    /// A mast arm spans the road. It may serve several approaches of one direction group.
    pub mast_arm: bool,
}

/// Limits for [`match_props`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchParams {
    /// Metres (horizontal) from the stop point, or from the junction corner, within which a prop may serve.
    pub max_distance: f32,
    /// Degrees a prop's facing may differ from the direction back along the approach.
    pub facing_tolerance: f32,
    /// Degrees between two approach headings that still count as one direction group for a shared mast arm.
    pub group_tolerance: f32,
}

impl Default for MatchParams {
    fn default() -> Self {
        Self { max_distance: 25.0, facing_tolerance: 60.0, group_tolerance: 45.0 }
    }
}

/// For each approach (same order as `approaches`), the index into `candidates` of the prop that serves it, or
/// `None` when no prop qualifies.
pub fn match_props(approaches: &[Approach], candidates: &[PropPose], params: &MatchParams) -> Vec<Option<usize>> {
    let mut result = vec![None; approaches.len()];
    let facing_cos = params.facing_tolerance.to_radians().cos();
    let mut pairs = Vec::new();
    for (approach, road) in approaches.iter().enumerate() {
        let site = match Site::new(road) {
            Some(site) => site,
            None => continue,
        };
        for (prop, pose) in candidates.iter().enumerate() {
            let reach = match site.reach(pose, facing_cos, params.max_distance) {
                Some(reach) => reach,
                None => continue,
            };
            pairs.push(Pair { reach, approach, prop });
        }
    }
    pairs.sort_by(|a, b| a.reach.total_cmp(&b.reach).then(a.approach.cmp(&b.approach)).then(a.prop.cmp(&b.prop)));
    for pair in &pairs {
        if result[pair.approach].is_some() {
            continue;
        }
        if !may_serve(approaches, candidates, &result, pair, params.group_tolerance) {
            continue;
        }
        result[pair.approach] = Some(pair.prop);
    }
    result
}

/// A valid (approach, prop) pair and how far the prop is from the approach's stop point.
struct Pair {
    reach: f32,
    approach: usize,
    prop: usize,
}

/// Whether the prop of `pair` can serve its approach now: unused, or a mast arm already serving approaches of the
/// same junction and direction group.
fn may_serve(
    approaches: &[Approach],
    candidates: &[PropPose],
    result: &[Option<usize>],
    pair: &Pair,
    group_tolerance: f32,
) -> bool {
    let first = match result.iter().position(|&served| served == Some(pair.prop)) {
        Some(first) => first,
        None => return true,
    };
    if !candidates[pair.prop].mast_arm {
        return false;
    }
    let (a, b) = (&approaches[first], &approaches[pair.approach]);
    a.junction == b.junction && a.heading.dot(b.heading) >= group_tolerance.to_radians().cos()
}

/// An approach reduced to the horizontal frame the rules are written in.
struct Site {
    stop: Vec3,
    corner: Vec3,
    heading: Vec3,
    right: Vec3,
}

impl Site {
    fn new(approach: &Approach) -> Option<Self> {
        let heading = planar(approach.heading).try_normalize()?;
        let stop = planar(approach.stop_position);
        Some(Self { stop, corner: stop + heading * STOP_LINE_DISTANCE, heading, right: right_of(heading) })
    }

    /// The distance from the stop point or the junction corner to `pose`, when `pose` qualifies for this
    /// approach: in reach, facing back along the approach, and on the right of the lane or across the junction.
    fn reach(&self, pose: &PropPose, facing_cos: f32, max_distance: f32) -> Option<f32> {
        let position = planar(pose.position);
        let reach = position.distance(self.stop).min(position.distance(self.corner));
        if reach > max_distance {
            return None;
        }
        let facing = planar(pose.forward).try_normalize()?;
        if facing.dot(-self.heading) < facing_cos {
            return None;
        }
        let offset = position - self.stop;
        let right_of_lane = offset.dot(self.right) > 0.0;
        let across_junction = offset.dot(self.heading) >= STOP_LINE_DISTANCE;
        if !right_of_lane && !across_junction {
            return None;
        }
        Some(reach)
    }
}

/// The horizontal part of `v` (height dropped).
fn planar(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}
