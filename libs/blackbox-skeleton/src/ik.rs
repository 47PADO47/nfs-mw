//! Two-bone inverse kinematics (an arm or a leg): bring the tip of a three-bone chain to a target, bending the middle
//! joint towards a pole point.

use glam::{Quat, Vec3};

use crate::error::{Error, Result};
use crate::skeleton::Skeleton;
use crate::transform::Transform;

const EPSILON: f32 = 1e-6;

/// The three bones of a limb: `mid` is a child of `root` and `tip` a child of `mid`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TwoBoneChain {
    pub root: usize,
    pub mid: usize,
    pub tip: usize,
}

/// Where the middle joint and the tip end up, in the space of the inputs. The tip reaches `target` when it is within
/// the limb's reach; otherwise the limb stretches straight towards it (or folds as far as it can).
#[must_use]
pub fn solve_positions(root: Vec3, mid: Vec3, tip: Vec3, target: Vec3, pole: Vec3) -> (Vec3, Vec3) {
    let upper = (mid - root).length();
    let lower = (tip - mid).length();
    if upper < EPSILON || lower < EPSILON {
        return (mid, tip);
    }
    let to_target = target - root;
    let direction = to_target.try_normalize().unwrap_or_else(|| (mid - root) / upper);
    let slack = 1e-4 * (upper + lower);
    let longest = upper + lower - slack;
    let shortest = ((upper - lower).abs() + slack).min(longest);
    let reach = to_target.length().clamp(shortest, longest);
    let cos_root = ((upper * upper + reach * reach - lower * lower) / (2.0 * upper * reach)).clamp(-1.0, 1.0);
    let sin_root = (1.0 - cos_root * cos_root).sqrt();
    let across = |v: Vec3| v - direction * v.dot(direction);
    let bend = [pole - root, mid - root]
        .into_iter()
        .map(across)
        .find_map(Vec3::try_normalize)
        .unwrap_or_else(|| direction.any_orthonormal_vector());
    let new_mid = root + direction * (upper * cos_root) + bend * (upper * sin_root);
    (new_mid, root + direction * reach)
}

/// The two rotations, about `root` and about `mid`, that carry the chain from where it is to the solution of
/// [`solve_positions`]. The second is applied after the first, about the middle joint's new position.
#[must_use]
pub fn solve_rotations(root: Vec3, mid: Vec3, tip: Vec3, target: Vec3, pole: Vec3) -> (Quat, Quat) {
    let (new_mid, new_tip) = solve_positions(root, mid, tip, target, pole);
    let (Some(old_upper), Some(new_upper)) = ((mid - root).try_normalize(), (new_mid - root).try_normalize()) else {
        return (Quat::IDENTITY, Quat::IDENTITY);
    };
    let first = Quat::from_rotation_arc(old_upper, new_upper);
    let tip_after_first = root + first * (tip - root);
    let (Some(old_lower), Some(new_lower)) =
        ((tip_after_first - new_mid).try_normalize(), (new_tip - new_mid).try_normalize())
    else {
        return (first, Quat::IDENTITY);
    };
    (first, Quat::from_rotation_arc(old_lower, new_lower))
}

/// Solves the limb in place on a local pose. `target` and `pole` are in component space (the space of the
/// skeleton's origin). `alpha` 0 leaves the pose, 1 reaches the target. With `tip_rotation` (component space) the
/// tip bone also takes that orientation. Bones are assumed to have unit scale.
pub fn apply_two_bone(
    skeleton: &Skeleton,
    local: &mut [Transform],
    chain: TwoBoneChain,
    target: Vec3,
    pole: Vec3,
    alpha: f32,
    tip_rotation: Option<Quat>,
) -> Result<()> {
    for bone in [chain.root, chain.mid, chain.tip] {
        if bone >= skeleton.len() {
            return Err(Error::BadBone(bone));
        }
    }
    let parents = |i: usize| skeleton.bones()[i].parent;
    if parents(chain.mid) != Some(chain.root) || parents(chain.tip) != Some(chain.mid) {
        return Err(Error::BadValue("the bones of a two-bone chain must be parent and child"));
    }
    let mut component = Vec::new();
    skeleton.component_transforms(local, &mut component)?;
    let (root, mid, tip) = (component[chain.root], component[chain.mid], component[chain.tip]);
    let (first, second) = solve_rotations(root.translation, mid.translation, tip.translation, target, pole);
    let first = Quat::IDENTITY.slerp(first, alpha);
    let second = Quat::IDENTITY.slerp(second, alpha);
    let root_parent = parents(chain.root).map_or(Quat::IDENTITY, |p| component[p].rotation);
    let new_root = (first * root.rotation).normalize();
    let new_mid = (second * first * mid.rotation).normalize();
    local[chain.root].rotation = (root_parent.inverse() * new_root).normalize();
    local[chain.mid].rotation = (new_root.inverse() * new_mid).normalize();
    if let Some(rotation) = tip_rotation {
        local[chain.tip].rotation = (new_mid.inverse() * rotation).normalize();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::tests::arm;

    const CHAIN: TwoBoneChain = TwoBoneChain { root: 1, mid: 2, tip: 3 };

    fn hand_and_elbow(skel: &Skeleton, local: &[Transform]) -> (Vec3, Vec3) {
        let mut c = Vec::new();
        skel.component_transforms(local, &mut c).unwrap();
        (c[3].translation, c[2].translation)
    }

    #[test]
    fn reaches_a_reachable_target_and_bends_towards_the_pole() {
        let skel = arm();
        let mut pose = skel.rest_pose();
        let target = Vec3::new(1.0, 2.0, 0.0);
        let pole = Vec3::new(0.0, 1.0, 5.0);
        apply_two_bone(&skel, &mut pose.local, CHAIN, target, pole, 1.0, None).unwrap();
        let (hand, elbow) = hand_and_elbow(&skel, &pose.local);
        assert!(hand.distance(target) < 1e-4, "{hand:?}");
        assert!(elbow.z > 0.5, "{elbow:?}");
        assert!((elbow.distance(Vec3::new(0.0, 1.0, 0.0)) - 1.0).abs() < 1e-4);
        assert!((elbow.distance(hand) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn stretches_towards_an_unreachable_target() {
        let skel = arm();
        let mut pose = skel.rest_pose();
        apply_two_bone(&skel, &mut pose.local, CHAIN, Vec3::new(10.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 1.0), 1.0, None)
            .unwrap();
        let (hand, _) = hand_and_elbow(&skel, &pose.local);
        assert!(hand.distance(Vec3::new(2.0, 1.0, 0.0)) < 1e-2, "{hand:?}");
    }

    #[test]
    fn alpha_zero_changes_nothing_and_tip_rotation_is_applied() {
        let skel = arm();
        let mut pose = skel.rest_pose();
        let before = pose.clone();
        apply_two_bone(&skel, &mut pose.local, CHAIN, Vec3::new(1.0, 2.0, 0.0), Vec3::Z, 0.0, None).unwrap();
        for (a, b) in pose.local.iter().zip(&before.local) {
            assert!(a.rotation.angle_between(b.rotation) < 1e-5);
        }
        let want = Quat::from_rotation_x(0.9);
        apply_two_bone(&skel, &mut pose.local, CHAIN, Vec3::new(1.0, 2.0, 0.0), Vec3::Z, 1.0, Some(want)).unwrap();
        let mut c = Vec::new();
        skel.component_transforms(&pose.local, &mut c).unwrap();
        assert!(c[3].rotation.angle_between(want) < 1e-4);
    }

    #[test]
    fn rejects_a_chain_that_is_not_connected() {
        let skel = arm();
        let mut pose = skel.rest_pose();
        let bad = TwoBoneChain { root: 1, mid: 3, tip: 2 };
        assert!(apply_two_bone(&skel, &mut pose.local, bad, Vec3::X, Vec3::Z, 1.0, None).is_err());
    }
}
