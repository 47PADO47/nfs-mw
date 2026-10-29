//! Leaning the rider forward when his arms cannot reach the grips.
//!
//! The animation is authored for one bike (the reference pose has the hands on that bike's grips). Another bike's grips can be
//! farther from the seat than the arms are long (a racer's clip-ons are below the hips), and the two-bone solve then stretches
//! the arm towards the grip and stops short. The original solves hands and body together with a full-body IK that moves the
//! spine; this does the part of that which matters here: it turns the spine forward about its own joints, one angle split evenly
//! over them, by the smallest angle that brings every shoulder within [`REACH_FRACTION`] of its arm's length of the grip; the
//! neck and head turn back against it so he keeps looking ahead. The pelvis stays on the seat and the legs are not moved by it.
//!
//! The numbers below are modelling choices, not data: [`REACH_FRACTION`] leaves the elbow bent about 20 degrees where the arm
//! could just reach, [`MAX_LEAN`] stops the body from folding over for a grip that is absurdly far (the contacts file is already
//! held to `contacts::MAX_REACH_M`), and [`STEP`] is the search's resolution before it is refined.

use blackbox_skeleton::{Error, Skeleton, Transform};
use glam::{Quat, Vec3};

use super::attach::Torso;

/// The part of its length a limb is asked to stretch to before the body leans for it.
pub const REACH_FRACTION: f32 = 0.93;
/// The most the body leans forward to reach, radians (about 72 degrees: the crouch of a racing bike).
pub const MAX_LEAN: f32 = 1.25;
/// The search's step, radians.
pub const STEP: f32 = 0.05;

/// The bones of the torso, found in the rider's skeleton.
#[derive(Debug, Clone, PartialEq)]
pub struct TorsoBones {
    /// The spine, root to tip.
    pub lean: Vec<usize>,
    /// The neck and head, root to tip.
    pub head: Vec<usize>,
    /// The part of the lean the head bones turn back by, in all.
    pub head_share: f32,
}

impl TorsoBones {
    /// Finds the bones `torso` names; says which one is missing.
    pub fn new(torso: &Torso, skeleton: &Skeleton) -> Result<Self, String> {
        let find = |names: &[String]| -> Result<Vec<usize>, String> {
            names
                .iter()
                .map(|name| skeleton.find(name).ok_or_else(|| format!("the rider has no bone {name}")))
                .collect()
        };
        Ok(Self { lean: find(&torso.lean)?, head: find(&torso.head)?, head_share: torso.head_share })
    }
}

/// An arm to bring to its grip: the shoulder bone, the arm's length and the place the tip is to be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Goal {
    pub root: usize,
    pub length: f32,
    pub tip: Vec3,
}

/// Turn the torso of the pose `local` forward by `angle` (radians, about `axis`, unit, in rider space; a positive turn takes the
/// top of the body forward): each spine bone turns about its own joint by an equal share, the head bones back by `head_share` of
/// the whole in all. The turns are about one axis, so they add up along the chain whatever order they are made in, and each
/// bone's local rotation can be worked out from the pose before any of them.
pub fn bend(
    skeleton: &Skeleton,
    local: &mut [Transform],
    torso: &TorsoBones,
    axis: Vec3,
    angle: f32,
) -> Result<(), Error> {
    let mut component = Vec::new();
    skeleton.component_transforms(local, &mut component)?;
    let mut turn = |bone: usize, share: f32| {
        let parent = skeleton.bones()[bone].parent.map_or(Quat::IDENTITY, |p| component[p].rotation);
        let about = Quat::from_axis_angle(axis, share);
        local[bone].rotation = (parent.inverse() * about * parent * local[bone].rotation).normalize();
    };
    for &bone in &torso.lean {
        turn(bone, angle / torso.lean.len() as f32);
    }
    for &bone in &torso.head {
        turn(bone, -angle * torso.head_share / torso.head.len() as f32);
    }
    Ok(())
}

/// How far the worst arm still falls short of reaching its grip with the torso leaned by `angle`: the largest of the distance from
/// a shoulder to its goal minus [`REACH_FRACTION`] of the arm's length (zero or less: every arm reaches).
fn shortfall(
    skeleton: &Skeleton,
    local: &[Transform],
    (torso, axis): (&TorsoBones, Vec3),
    goals: &[Goal],
    angle: f32,
) -> Result<f32, Error> {
    let mut turned = local.to_vec();
    bend(skeleton, &mut turned, torso, axis, angle)?;
    let mut component = Vec::new();
    skeleton.component_transforms(&turned, &mut component)?;
    Ok(goals
        .iter()
        .map(|g| component[g.root].translation.distance(g.tip) - REACH_FRACTION * g.length)
        .fold(f32::MIN, f32::max))
}

/// The forward lean (radians, 0 to [`MAX_LEAN`]) of the torso of the pose `local` about `axis` (unit, rider space; zero for no
/// lean) that brings every arm of `goals` within reach of its grip: 0 when they all can already, the smallest such angle
/// otherwise, and the angle that comes nearest when none does.
pub fn solve(
    skeleton: &Skeleton,
    local: &[Transform],
    torso: &TorsoBones,
    axis: Vec3,
    goals: &[Goal],
) -> Result<f32, Error> {
    if axis == Vec3::ZERO || goals.is_empty() {
        return Ok(0.0);
    }
    let missing = |angle: f32| shortfall(skeleton, local, (torso, axis), goals, angle);
    let at_rest = missing(0.0)?;
    if !at_rest.is_finite() || at_rest <= 0.0 {
        return Ok(0.0);
    }
    let (mut before, mut nearest) = (0.0_f32, (at_rest, 0.0_f32));
    for n in 1..=(MAX_LEAN / STEP).ceil() as usize {
        let angle = (n as f32 * STEP).min(MAX_LEAN);
        let short = missing(angle)?;
        if short <= 0.0 {
            // The first angle that reaches: refine between it and the one before.
            let (mut low, mut high) = (before, angle);
            for _ in 0..12 {
                let middle = 0.5 * (low + high);
                match missing(middle)? <= 0.0 {
                    true => high = middle,
                    false => low = middle,
                }
            }
            return Ok(high);
        }
        if short < nearest.0 {
            nearest = (short, angle);
        }
        before = angle;
    }
    Ok(nearest.1)
}

#[cfg(test)]
#[path = "reach_tests.rs"]
mod tests;
