//! Posing a skeleton and skinning a model with the pose. Spec: `docs/formats/chameleon-driver-animation.md`.
//!
//! A skeleton stores each bone's **world** rest position and **world** rest orientation (a child's offset is along its
//! parent's +x axis in the parent's frame). A clip stores, per bone, a small rotation expressed in the **rest world axes**
//! (x left, y up, z forward: a turn about y is a turn of the bone about the up axis, whatever the bone's own axes are). The
//! bone's local rotation is `parent_rest^-1 * delta * rest`. Positions of the bones follow from the rotations: a child sits
//! at its parent's position plus the rest offset, turned by the parent's posed orientation. Checked on all 28 clips: with
//! this reading the hand bones land within 1 mm of the hand positions the clips store.

use crate::clip::Clip;
use crate::quat::{self, IDENTITY, Quat};
use crate::skeleton::Skeleton;

/// Where every bone is and how it faces.
#[derive(Debug, Clone, PartialEq)]
pub struct Pose {
    pub rotations: Vec<Quat>,
    pub positions: Vec<[f32; 3]>,
}

/// Four bone indices with their weights (the weights sum to 255 or are all zero).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skin {
    pub bones: [u8; 4],
    pub weights: [u8; 4],
}

impl Pose {
    pub fn new(bones: usize) -> Self {
        Self { rotations: vec![IDENTITY; bones], positions: vec![[0.0; 3]; bones] }
    }
}

/// The rotation of each bone relative to its rest, in the rest world axes, to be applied by [`Skeleton::pose`].
pub type Deltas = Vec<Quat>;

impl Skeleton {
    /// A set of deltas that leave every bone at rest.
    pub fn rest_deltas(&self) -> Deltas {
        vec![IDENTITY; self.bones.len()]
    }

    /// Set `deltas` to the rotations the clip has at `time` seconds for the bones it turns, replacing those of the bones
    /// it has a track for and leaving the others as they are.
    pub fn sample(&self, clip: &Clip, time: f32, deltas: &mut [Quat]) {
        for (bone, delta) in deltas.iter_mut().enumerate().take(self.bones.len()) {
            if let Some(q) = clip.rotation(bone, time) {
                *delta = q;
            }
        }
    }

    /// The local rotation of `bone` when it is turned by `delta` (in the rest world axes): from its parent's posed
    /// orientation to its own.
    fn local(&self, bone: usize, delta: Quat) -> Quat {
        let own = self.bones[bone].rotation;
        let turned = quat::mul(delta, own);
        match self.bones[bone].parent {
            Some(parent) => quat::mul(quat::conjugate(self.bones[parent].rotation), turned),
            None => turned,
        }
    }

    /// Pose the skeleton: every bone turned by its delta (in `deltas`, one per bone, in the rest world axes), the root where
    /// it rests.
    pub fn pose(&self, deltas: &[Quat], out: &mut Pose) {
        out.rotations.resize(self.bones.len(), IDENTITY);
        out.positions.resize(self.bones.len(), [0.0; 3]);
        for (n, bone) in self.bones.iter().enumerate() {
            let delta = deltas.get(n).copied().unwrap_or(IDENTITY);
            let local = self.local(n, delta);
            let Some(parent) = bone.parent else {
                out.rotations[n] = quat::normalize(local);
                out.positions[n] = bone.position;
                continue;
            };
            let from = &self.bones[parent];
            let offset =
                quat::rotate(quat::conjugate(from.rotation), [0, 1, 2].map(|k| bone.position[k] - from.position[k]));
            let moved = quat::rotate(out.rotations[parent], offset);
            out.positions[n] = [0, 1, 2].map(|k| out.positions[parent][k] + moved[k]);
            out.rotations[n] = quat::normalize(quat::mul(out.rotations[parent], local));
        }
    }

    /// The position of `point` (a point of the model at rest) once it is skinned by `skin` to `pose`. A vertex with no
    /// weight stays where it is.
    pub fn skin_point(&self, pose: &Pose, skin: &Skin, point: [f32; 3]) -> [f32; 3] {
        let total: u32 = skin.weights.iter().map(|&w| u32::from(w)).sum();
        if total == 0 {
            return point;
        }
        let mut out = [0.0f32; 3];
        for (&bone, &weight) in skin.bones.iter().zip(&skin.weights) {
            let bone = usize::from(bone);
            if weight == 0 || bone >= self.bones.len() {
                continue;
            }
            let rest = &self.bones[bone];
            let local = quat::rotate(quat::conjugate(rest.rotation), [0, 1, 2].map(|k| point[k] - rest.position[k]));
            let moved = quat::rotate(pose.rotations[bone], local);
            let w = f32::from(weight) / total as f32;
            for k in 0..3 {
                out[k] += w * (pose.positions[bone][k] + moved[k]);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::Bone;

    /// A three-bone arm along +x from the origin: a root at 0, an elbow at 1 and a hand at 2 metres, each bone facing
    /// along its +x axis (identity orientations).
    fn arm() -> Skeleton {
        let bone =
            |parent: Option<usize>, x: f32| Bone { hash: 0, parent, position: [x, 0.0, 0.0], rotation: IDENTITY };
        Skeleton { bones: vec![bone(None, 0.0), bone(Some(0), 1.0), bone(Some(1), 2.0)] }
    }

    fn quarter_turn_about_z() -> Quat {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        [0.0, 0.0, s, s]
    }

    #[test]
    fn a_rest_pose_leaves_every_bone_where_it_is() {
        let skeleton = arm();
        let mut pose = Pose::new(3);
        skeleton.pose(&skeleton.rest_deltas(), &mut pose);
        for (n, bone) in skeleton.bones.iter().enumerate() {
            assert_eq!(pose.positions[n], bone.position);
        }
        let skin = Skin { bones: [1, 2, 0, 0], weights: [128, 127, 0, 0] };
        let moved = skeleton.skin_point(&pose, &skin, [1.5, 0.2, 0.0]);
        assert!(moved.iter().zip([1.5, 0.2, 0.0]).all(|(a, b)| (a - b).abs() < 1e-5), "{moved:?}");
    }

    #[test]
    fn turning_the_elbow_swings_the_hand_about_it() {
        let skeleton = arm();
        let mut deltas = skeleton.rest_deltas();
        deltas[1] = quarter_turn_about_z();
        let mut pose = Pose::new(3);
        skeleton.pose(&deltas, &mut pose);
        // The hand is a metre from the elbow; turned a quarter about z it is a metre up (+y) from it.
        let hand = pose.positions[2];
        assert!((hand[0] - 1.0).abs() < 1e-5 && (hand[1] - 1.0).abs() < 1e-5, "{hand:?}");
        assert_eq!(pose.positions[1], [1.0, 0.0, 0.0], "the elbow does not move");
    }

    #[test]
    fn a_vertex_follows_its_bone_and_blends_between_two() {
        let skeleton = arm();
        let mut deltas = skeleton.rest_deltas();
        deltas[1] = quarter_turn_about_z();
        let mut pose = Pose::new(3);
        skeleton.pose(&deltas, &mut pose);
        let whole = Skin { bones: [1, 0, 0, 0], weights: [255, 0, 0, 0] };
        let moved = skeleton.skin_point(&pose, &whole, [1.5, 0.0, 0.0]);
        assert!((moved[0] - 1.0).abs() < 1e-5 && (moved[1] - 0.5).abs() < 1e-5, "{moved:?}");
        let half = Skin { bones: [1, 0, 0, 0], weights: [128, 127, 0, 0] };
        let mixed = skeleton.skin_point(&pose, &half, [1.5, 0.0, 0.0]);
        assert!(mixed[1] > 0.2 && mixed[1] < 0.3, "{mixed:?}");
    }

    #[test]
    fn a_vertex_with_no_weight_stays() {
        let skeleton = arm();
        let pose = Pose::new(3);
        let skin = Skin { bones: [0; 4], weights: [0; 4] };
        assert_eq!(skeleton.skin_point(&pose, &skin, [3.0, 2.0, 1.0]), [3.0, 2.0, 1.0]);
    }
}
