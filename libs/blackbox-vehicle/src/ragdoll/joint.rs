//! The joints of the ragdoll: a point that two segments share, and the limits of the turn between them.
//!
//! The limits are those of a swing-and-twist joint. Each segment carries a constraint frame (a rotation in its body frame); the
//! twist axis is the frame's X axis. The turn of the child's frame against the parent's is split into a twist about X and a swing
//! about Y and Z, and each is held to its limit.

use glam::{Quat, Vec3};

use super::body::Body;

/// A joint between two segments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Joint {
    pub parent: usize,
    pub child: usize,
    /// The shared point, in each body's frame.
    pub anchor_parent: Vec3,
    pub anchor_child: Vec3,
    /// The constraint frames, in each body's frame.
    pub frame_parent: Quat,
    pub frame_child: Quat,
    pub limits: JointLimits,
}

/// How far the child may turn from the parent (rad): the swing about the frame's Z axis and about its Y axis, and the twist about
/// its X axis. Pi leaves the axis free, 0 locks it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointLimits {
    pub swing_z: f32,
    pub swing_y: f32,
    pub twist: f32,
}

/// Two different bodies of the list, mutably.
fn pair(bodies: &mut [Body], a: usize, b: usize) -> (&mut Body, &mut Body) {
    debug_assert_ne!(a, b);
    match a < b {
        true => {
            let (low, high) = bodies.split_at_mut(b);
            (&mut low[a], &mut high[0])
        }
        false => {
            let (low, high) = bodies.split_at_mut(a);
            (&mut high[0], &mut low[b])
        }
    }
}

/// Bring the two anchors together, each segment moving by its share of the gap.
pub(super) fn solve_point(bodies: &mut [Body], joint: &Joint) {
    let (parent, child) = pair(bodies, joint.parent, joint.child);
    let (pa, pb) = (parent.point(joint.anchor_parent), child.point(joint.anchor_child));
    let gap = pb - pa;
    let Some(n) = gap.try_normalize() else { return };
    let (ra, rb) = (pa - parent.x, pb - child.x);
    let share = parent.inverse_mass_at(ra, n) + child.inverse_mass_at(rb, n);
    let push = n * (gap.length() / share);
    parent.push_at(ra, push);
    child.push_at(rb, -push);
}

/// The turn of the child's constraint frame against the parent's, as the swing about Z, the swing about Y and the twist about X
/// (rad), and the parent's constraint frame in the world.
pub(super) fn deviation(bodies: &[Body], joint: &Joint) -> (Vec3, Quat) {
    let frame = bodies[joint.parent].q * joint.frame_parent;
    let mut rel = frame.inverse() * (bodies[joint.child].q * joint.frame_child);
    if rel.w < 0.0 {
        rel = -rel;
    }
    let twist = 2.0 * rel.x.atan2(rel.w);
    let twist_q = Quat::from_rotation_x(twist);
    let swing = (rel * twist_q.inverse()).to_scaled_axis();
    (Vec3::new(swing.z, swing.y, twist), frame)
}

/// Take a share `rate` (0 to 1) of the angular velocity of the child against the parent's away, each segment by its share: a joint
/// that is not free to swing for ever.
pub(super) fn damp(bodies: &mut [Body], joint: &Joint, rate: f32) {
    let (parent, child) = pair(bodies, joint.parent, joint.child);
    let relative = child.w - parent.w;
    let Some(n) = relative.try_normalize() else { return };
    let (wp, wc) = (n.dot(parent.inv_world() * n), n.dot(child.inv_world() * n));
    let total = wp + wc;
    if total <= 0.0 {
        return;
    }
    let lambda = relative.length() * rate.clamp(0.0, 1.0) / total;
    let (child_spin, parent_spin) = (child.inv_world() * n * lambda, parent.inv_world() * n * lambda);
    // The shared point keeps the velocity it has: each centre moves to make up what its turning takes from the point.
    let (arm_parent, arm_child) =
        (parent.point(joint.anchor_parent) - parent.x, child.point(joint.anchor_child) - child.x);
    child.w -= child_spin;
    child.v += child_spin.cross(arm_child);
    parent.w += parent_spin;
    parent.v -= parent_spin.cross(arm_parent);
}

/// The child's frame in the parent's for a deviation `d` (the swing about Z, the swing about Y, the twist about X).
fn build(d: Vec3) -> Quat {
    Quat::from_scaled_axis(Vec3::new(0.0, d.y, d.x)) * Quat::from_rotation_x(d.z)
}

/// Turn the child and the parent, each by its share, until the child's frame is inside the limits (`slack` is added to them).
pub(super) fn solve_limits(bodies: &mut [Body], joint: &Joint, slack: Vec3) {
    let (dev, frame) = deviation(bodies, joint);
    let limit = Vec3::new(joint.limits.swing_z, joint.limits.swing_y, joint.limits.twist) + slack;
    let held = dev.clamp(-limit, limit);
    if held == dev {
        return;
    }
    // In the parent's constraint frame, the child must turn by the move from where it is to where it is held.
    let fix = build(held) * build(dev).inverse();
    let turn = frame * fix.to_scaled_axis();
    let angle = turn.length();
    let Some(n) = turn.try_normalize() else { return };
    let (parent, child) = pair(bodies, joint.parent, joint.child);
    let (wp, wc) = (n.dot(parent.inv_world() * n), n.dot(child.inv_world() * n));
    let total = wp + wc;
    if total <= 0.0 {
        return;
    }
    let lambda = angle / total;
    let (child_turn, parent_turn) = (child.inv_world() * n * lambda, parent.inv_world() * n * lambda);
    child.rotate(child_turn);
    parent.rotate(-parent_turn);
}

/// Turn the child and the parent, each by its share, so that the child's orientation goes a fraction `gain` (0 to 1) of the way to
/// `target`, the orientation the child should have in the parent's body frame: the stiff angular drive of a body that follows an
/// animation.
pub(super) fn solve_drive(bodies: &mut [Body], joint: &Joint, target: Quat, gain: f32) {
    let (parent, child) = pair(bodies, joint.parent, joint.child);
    let mut error = (parent.q * target) * child.q.inverse();
    if error.w < 0.0 {
        error = -error;
    }
    let turn = error.to_scaled_axis();
    let angle = turn.length();
    let Some(n) = turn.try_normalize() else { return };
    let (wp, wc) = (n.dot(parent.inv_world() * n), n.dot(child.inv_world() * n));
    let total = wp + wc;
    if total <= 0.0 {
        return;
    }
    let lambda = gain.clamp(0.0, 1.0) * angle / total;
    let (child_turn, parent_turn) = (child.inv_world() * n * lambda, parent.inv_world() * n * lambda);
    child.rotate(child_turn);
    parent.rotate(-parent_turn);
}
