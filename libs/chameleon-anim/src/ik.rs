//! A two-bone reach (an arm to a grip). Spec: `docs/formats/chameleon-driver-animation.md`.
//!
//! The 2012 game puts a driver's hands on the wheel by solving the arms to targets after the clips have posed the body.
//! [`reach`] does the same for any chain of three bones (shoulder, elbow, wrist): it turns the first two so the third lands
//! on the target, bending the elbow to the side it already bends.

use crate::pose::{Deltas, Pose};
use crate::quat::{self, IDENTITY, Quat};
use crate::skeleton::Skeleton;

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}

fn scaled(a: [f32; 3], s: f32) -> [f32; 3] {
    a.map(|c| c * s)
}

fn plus(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn unit(a: [f32; 3]) -> Option<[f32; 3]> {
    let l = length(a);
    (l > 1e-6).then(|| scaled(a, 1.0 / l))
}

/// The rotation that takes the unit vector `from` to the unit vector `to` by the shortest way.
pub fn arc(from: [f32; 3], to: [f32; 3]) -> Quat {
    let d = dot(from, to);
    if d > 0.999_999 {
        return IDENTITY;
    }
    if d < -0.999_999 {
        // Half a turn about any axis across `from`.
        let axis = unit([-from[1], from[0], 0.0]).or_else(|| unit([0.0, -from[2], from[1]])).unwrap_or([1.0, 0.0, 0.0]);
        return [axis[0], axis[1], axis[2], 0.0];
    }
    let cross =
        [from[1] * to[2] - from[2] * to[1], from[2] * to[0] - from[0] * to[2], from[0] * to[1] - from[1] * to[0]];
    quat::normalize([cross[0], cross[1], cross[2], 1.0 + d])
}

/// Turn the chain `shoulder`, `elbow`, `wrist` (each the next one's parent) so the wrist reaches `target`, as far as the
/// arm's length allows. `deltas` are the ones `pose` was made from; the shoulder's and the elbow's are changed, and `pose`
/// is made again from them. Returns how far the wrist is from the target afterwards, in metres.
pub fn reach(skeleton: &Skeleton, deltas: &mut Deltas, pose: &mut Pose, chain: [usize; 3], target: [f32; 3]) -> f32 {
    let [shoulder, elbow, wrist] = chain;
    skeleton.pose(deltas, pose);
    let (root, mid, tip) = (pose.positions[shoulder], pose.positions[elbow], pose.positions[wrist]);
    let (upper, lower) = (length(sub(mid, root)), length(sub(tip, mid)));
    let to_target = sub(target, root);
    let (Some(aim), true) = (unit(to_target), upper > 1e-6 && lower > 1e-6) else { return length(sub(target, tip)) };
    // The reachable distance: the arm cannot straighten past its length nor fold shut.
    let far = length(to_target).clamp((upper - lower).abs() + 1e-4, upper + lower - 1e-4);
    // The elbow lies on the circle of points `upper` from the shoulder and `lower` from the reachable wrist; it keeps the
    // side it was on.
    let along = (upper * upper - lower * lower + far * far) / (2.0 * far);
    let height = (upper * upper - along * along).max(0.0).sqrt();
    let bent = sub(mid, root);
    let off_axis = sub(bent, scaled(aim, dot(bent, aim)));
    let side = unit(off_axis).or_else(|| unit(sub([0.0, 1.0, 0.0], scaled(aim, aim[1])))).unwrap_or([0.0, 0.0, 1.0]);
    let new_mid = plus(root, plus(scaled(aim, along), scaled(side, height)));
    let new_tip = plus(root, scaled(aim, far));
    let (Some(old_upper), Some(new_upper)) = (unit(sub(mid, root)), unit(sub(new_mid, root))) else { return 0.0 };
    let turn_upper = arc(old_upper, new_upper);
    let (Some(old_lower), Some(new_lower)) =
        (unit(quat::rotate(turn_upper, sub(tip, mid))), unit(sub(new_tip, new_mid)))
    else {
        return 0.0;
    };
    let turn_lower = arc(old_lower, new_lower);
    // New world orientations, back to deltas: the inverse of `Skeleton::pose`'s `parent_rest^-1 * delta * rest`.
    let new_shoulder = quat::mul(turn_upper, pose.rotations[shoulder]);
    let new_elbow = quat::mul(turn_lower, quat::mul(turn_upper, pose.rotations[elbow]));
    let delta_of = |bone: usize, world: Quat, parent_world: Quat| {
        let parent_rest = skeleton.bones[bone].parent.map_or(IDENTITY, |p| skeleton.bones[p].rotation);
        let local = quat::mul(quat::conjugate(parent_world), world);
        quat::normalize(quat::mul(parent_rest, quat::mul(local, quat::conjugate(skeleton.bones[bone].rotation))))
    };
    let shoulder_parent = skeleton.bones[shoulder].parent.map_or(IDENTITY, |p| pose.rotations[p]);
    deltas[shoulder] = delta_of(shoulder, new_shoulder, shoulder_parent);
    deltas[elbow] = delta_of(elbow, new_elbow, new_shoulder);
    skeleton.pose(deltas, pose);
    length(sub(target, pose.positions[wrist]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::Bone;

    /// An arm along +x: shoulder at the origin, elbow at 1, wrist at 2, with a bent rest (the elbow below the line).
    fn arm() -> Skeleton {
        let bone = |parent: Option<usize>, p: [f32; 3]| Bone { hash: 0, parent, position: p, rotation: IDENTITY };
        Skeleton { bones: vec![bone(None, [0.0; 3]), bone(Some(0), [1.0, 0.0, 0.0]), bone(Some(1), [2.0, 0.0, 0.0])] }
    }

    fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        length(sub(a, b))
    }

    #[test]
    fn the_wrist_lands_on_a_target_in_reach() {
        let skeleton = arm();
        let mut deltas = skeleton.rest_deltas();
        let mut pose = Pose::new(3);
        let target = [1.2, 0.9, 0.3];
        let miss = reach(&skeleton, &mut deltas, &mut pose, [0, 1, 2], target);
        assert!(miss < 1e-3, "{miss}");
        assert!(distance(pose.positions[2], target) < 1e-3);
        // The bones keep their lengths.
        assert!((distance(pose.positions[0], pose.positions[1]) - 1.0).abs() < 1e-4);
        assert!((distance(pose.positions[1], pose.positions[2]) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn a_chain_with_turned_rest_axes_reaches_too() {
        // Every bone faces along +y at rest (its +x axis is the world +y), the way the characters' arms face their children.
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let bone = |parent: Option<usize>, y: f32| Bone {
            hash: 0,
            parent,
            position: [0.0, y, 0.0],
            rotation: [0.0, 0.0, s, s],
        };
        let skeleton = Skeleton { bones: vec![bone(None, 0.0), bone(Some(0), 1.0), bone(Some(1), 2.0)] };
        let mut deltas = skeleton.rest_deltas();
        let mut pose = Pose::new(3);
        let target = [0.8, 1.0, 0.5];
        let miss = reach(&skeleton, &mut deltas, &mut pose, [0, 1, 2], target);
        assert!(miss < 1e-3, "{miss}");
        assert!((distance(pose.positions[1], pose.positions[2]) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn a_target_out_of_reach_stretches_towards_it() {
        let skeleton = arm();
        let mut deltas = skeleton.rest_deltas();
        let mut pose = Pose::new(3);
        let miss = reach(&skeleton, &mut deltas, &mut pose, [0, 1, 2], [0.0, 5.0, 0.0]);
        assert!((miss - 3.0).abs() < 1e-2, "{miss}");
        assert!(pose.positions[2][1] > 1.9, "{:?}", pose.positions[2]);
    }

    #[test]
    fn a_target_where_the_wrist_already_is_changes_nothing() {
        let skeleton = arm();
        let mut deltas = skeleton.rest_deltas();
        let mut pose = Pose::new(3);
        let miss = reach(&skeleton, &mut deltas, &mut pose, [0, 1, 2], [2.0, 0.0, 0.0]);
        assert!(miss < 1e-3);
        assert!(deltas.iter().all(|d| quat::angle_degrees(*d) < 1.5), "{deltas:?}");
    }

    #[test]
    fn the_arc_between_two_directions_turns_one_into_the_other() {
        let q = arc([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let r = quat::rotate(q, [1.0, 0.0, 0.0]);
        assert!(distance(r, [0.0, 1.0, 0.0]) < 1e-5, "{r:?}");
        let back = arc([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);
        assert!(distance(quat::rotate(back, [1.0, 0.0, 0.0]), [-1.0, 0.0, 0.0]) < 1e-5);
    }
}
