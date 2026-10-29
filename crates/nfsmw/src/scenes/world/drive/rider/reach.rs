//! Leaning the rider forward when his arms cannot reach the grips.
//!
//! The animation is authored for one bike (the reference pose has the hands on that bike's grips). Another bike's grips can be
//! farther from the seat than the arms are long, and the two-bone solve then stretches the arm towards the grip and stops short.
//! The original solves hands and body together with a full-body IK that moves the spine; this does the one part of that which
//! matters here: it turns the body forward about the pelvis (the bone the shift moves), by just the angle that brings the
//! shoulders of the limbs that are too short to their goals, one angle for all limbs, in the least-squares sense.
//!
//! The two numbers below are modelling choices, not data: [`REACH_FRACTION`] keeps the elbow a little bent where the arm could
//! just reach, and [`MAX_LEAN`] stops the body from folding over for a grip that is absurdly far (the contacts file is already
//! held to [`super::contacts::MAX_REACH_M`]).

use blackbox_skeleton::Transform;
use glam::Vec3;

use super::pose::Resolved;

/// The part of its length a limb is asked to stretch to before the body leans for it.
pub const REACH_FRACTION: f32 = 0.95;
/// The most the body leans forward to reach, radians (about 29 degrees).
pub const MAX_LEAN: f32 = 0.5;

/// The forward lean (radians, 0 to [`MAX_LEAN`]) about `axis` (unit, rider space, a positive turn takes the top of the body
/// forward; zero for no lean) through the shift bone that brings every limb whose goal is out of reach as near as one angle can.
/// `component` are the rider's component transforms, `targets` the places the limbs reach for (as for the solve).
pub fn lean(resolved: &Resolved, component: &[Transform], targets: &[Vec3], axis: Vec3) -> f32 {
    if axis == Vec3::ZERO {
        return 0.0;
    }
    let pivot = component[resolved.shift_bone].translation;
    let (mut wanted, mut moved) = (0.0_f32, 0.0_f32);
    for limb in &resolved.limbs {
        let Some(target) = targets.get(limb.target) else { continue };
        let at = |bone: usize| component[bone].translation;
        let (root, mid, tip) = (at(limb.chain.root), at(limb.chain.mid), at(limb.chain.tip));
        let to_goal = *target + limb.offset - root;
        let excess = to_goal.length() - REACH_FRACTION * (mid.distance(root) + tip.distance(mid));
        if !excess.is_finite() || excess <= 0.0 {
            continue;
        }
        // A turn of one radian about the axis moves the root by `axis x (root - pivot)`.
        let step = axis.cross(root - pivot);
        wanted += (to_goal.normalize_or_zero() * excess).dot(step);
        moved += step.dot(step);
    }
    if moved < 1e-6 {
        return 0.0;
    }
    (wanted / moved).clamp(0.0, MAX_LEAN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_skeleton::TwoBoneChain;

    use super::super::pose::Limb;

    /// A rider reduced to a pivot (0) and one arm, straight up from a root 0.5 m above the pivot, whose two bones are 0.3 m long. Forward is +x,
    /// and the lean axis +y takes the top of the body forward.
    fn arm() -> (Resolved, Vec<Transform>) {
        let at = |x: f32, z: f32| Transform::from_rt(glam::Quat::IDENTITY, Vec3::new(x, 0.0, z));
        let limb = Limb { chain: TwoBoneChain { root: 1, mid: 2, tip: 3 }, pole: 0, target: 0, offset: Vec3::ZERO };
        (Resolved { limbs: vec![limb], shift_bone: 0 }, vec![at(0.0, 0.0), at(0.0, 0.5), at(0.0, 0.8), at(0.0, 1.1)])
    }

    #[test]
    fn a_goal_the_arm_can_reach_needs_no_lean() {
        let (resolved, component) = arm();
        // The arm is 0.6 m long; the goal is 0.5 m from the shoulder.
        assert_eq!(lean(&resolved, &component, &[Vec3::new(0.5, 0.0, 0.5)], Vec3::Y), 0.0);
        // No axis, no lean.
        assert_eq!(lean(&resolved, &component, &[Vec3::new(2.0, 0.0, 0.5)], Vec3::ZERO), 0.0);
        // A limb with no target is left alone.
        assert_eq!(lean(&resolved, &component, &[], Vec3::Y), 0.0);
    }

    #[test]
    fn a_goal_too_far_ahead_is_closed_by_leaning_the_shoulder_forward() {
        let (resolved, component) = arm();
        // The goal is 0.65 m ahead of the shoulder: 0.08 m beyond 0.95 of the arm's 0.6 m. The shoulder is 0.5 m above the
        // pivot, so a turn of 0.08 / 0.5 radians brings it there.
        let angle = lean(&resolved, &component, &[Vec3::new(0.65, 0.0, 0.5)], Vec3::Y);
        assert!((angle - 0.16).abs() < 1e-4, "{angle}");
        // The shoulder, turned by that angle about the pivot, is that much nearer the goal.
        let turned = glam::Quat::from_axis_angle(Vec3::Y, angle) * Vec3::new(0.0, 0.0, 0.5);
        assert!((Vec3::new(0.65, 0.0, 0.5).distance(turned) - 0.57).abs() < 5e-3, "{turned:?}");
    }

    #[test]
    fn the_lean_is_held_to_its_limit_and_never_goes_backwards() {
        let (resolved, component) = arm();
        assert_eq!(lean(&resolved, &component, &[Vec3::new(1.4, 0.0, 0.5)], Vec3::Y), MAX_LEAN);
        // A goal out of reach behind the rider would need a lean back: not taken.
        assert_eq!(lean(&resolved, &component, &[Vec3::new(-0.9, 0.0, 0.5)], Vec3::Y), 0.0);
        assert_eq!(lean(&resolved, &component, &[Vec3::splat(f32::NAN)], Vec3::Y), 0.0);
    }
}
