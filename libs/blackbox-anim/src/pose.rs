//! Poses: the scale, rotation and translation of every bone, and the flat DOF buffer that the channel codecs fill.
//!
//! The DOF buffer holds [`DOFS_PER_BONE`] floats per bone, bone after bone. The slots are:
//!
//! | Slots | Value |
//! |---|---|
//! | 0 to 2 | scale x, y, z |
//! | 3 | unused |
//! | 4 to 7 | rotation quaternion x, y, z, w |
//! | 8 to 10 | translation x, y, z |
//! | 11 | unused |
//!
//! A codec writes only the slots of the channel it drives. So a buffer is first filled from the bind pose (with
//! [`Pose::write_dofs`] on [`crate::Skeleton::bind_pose`]), and a channel that does not touch a slot leaves the bind
//! value in it.

use glam::{Mat4, Quat, Vec3};

/// The floats per bone in a DOF buffer. Slots 3 and 11 are unused.
pub const DOFS_PER_BONE: usize = 12;

const SCALE: usize = 0;
const ROTATION: usize = 4;
const TRANSLATION: usize = 8;

/// One bone's transform: a scale, then a rotation, then a translation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sqt {
    /// The scale along the bone's own axes.
    pub scale: Vec3,
    /// The rotation, a quaternion.
    pub rotation: Quat,
    /// The translation, in the parent's frame.
    pub translation: Vec3,
}

impl Sqt {
    /// No scale, no rotation and no translation.
    pub const IDENTITY: Self = Self { scale: Vec3::ONE, rotation: Quat::IDENTITY, translation: Vec3::ZERO };

    /// The local matrix `T * R * S`: the scale acts first, in the bone's own frame, then the rotation, then the
    /// translation. Multiply it by the parent's world matrix to get the bone's world matrix.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.translation) * Mat4::from_quat(self.rotation) * Mat4::from_scale(self.scale)
    }
}

/// The transform of every bone of a skeleton, in the skeleton's bone order.
#[derive(Debug, Clone, PartialEq)]
pub struct Pose {
    /// One transform per bone.
    pub bones: Vec<Sqt>,
}

impl Pose {
    /// Reads a DOF buffer of [`DOFS_PER_BONE`] floats per bone. The quaternion is normalised, and a zero quaternion
    /// becomes the identity rotation. Panics unless `dofs.len()` is a multiple of [`DOFS_PER_BONE`].
    pub fn from_dofs(dofs: &[f32]) -> Self {
        assert!(
            dofs.len().is_multiple_of(DOFS_PER_BONE),
            "a DOF buffer holds {DOFS_PER_BONE} floats per bone; got {} floats",
            dofs.len()
        );
        let (slots, _) = dofs.as_chunks::<DOFS_PER_BONE>();
        let bones = slots.iter().map(bone_from_dofs).collect();
        Self { bones }
    }

    /// Writes every bone's scale, rotation and translation into a DOF buffer. The unused slots are not written.
    /// Panics unless `out` holds exactly [`DOFS_PER_BONE`] floats per bone.
    pub fn write_dofs(&self, out: &mut [f32]) {
        assert_eq!(
            out.len(),
            self.bones.len() * DOFS_PER_BONE,
            "a pose of {} bones needs a DOF buffer of {} floats",
            self.bones.len(),
            self.bones.len() * DOFS_PER_BONE
        );
        let (slots, _) = out.as_chunks_mut::<DOFS_PER_BONE>();
        for (bone, slots) in self.bones.iter().zip(slots) {
            write_bone(bone, slots);
        }
    }
}

fn bone_from_dofs(slots: &[f32; DOFS_PER_BONE]) -> Sqt {
    Sqt {
        scale: vec3_at(slots, SCALE),
        rotation: unit_or_identity(Quat::from_xyzw(
            slots[ROTATION],
            slots[ROTATION + 1],
            slots[ROTATION + 2],
            slots[ROTATION + 3],
        )),
        translation: vec3_at(slots, TRANSLATION),
    }
}

fn write_bone(bone: &Sqt, slots: &mut [f32; DOFS_PER_BONE]) {
    slots[SCALE..SCALE + 3].copy_from_slice(&bone.scale.to_array());
    slots[ROTATION..ROTATION + 4].copy_from_slice(&bone.rotation.to_array());
    slots[TRANSLATION..TRANSLATION + 3].copy_from_slice(&bone.translation.to_array());
}

fn vec3_at(slots: &[f32], at: usize) -> Vec3 {
    Vec3::new(slots[at], slots[at + 1], slots[at + 2])
}

/// The quaternion scaled to unit length. A zero (or NaN) quaternion has no direction, so it becomes the identity.
fn unit_or_identity(rotation: Quat) -> Quat {
    let length_sq = rotation.length_squared();
    if length_sq.is_nan() || length_sq <= 0.0 {
        return Quat::IDENTITY;
    }
    rotation / length_sq.sqrt()
}
