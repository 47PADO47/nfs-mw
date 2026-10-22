use std::f32::consts::FRAC_PI_2;

use glam::{Mat4, Quat, Vec3};

use crate::{DOFS_PER_BONE, Pose, Sqt};

/// Two bones: a scaled, rotated and moved first bone, and an identity second bone.
fn two_bones() -> Pose {
    let first = Sqt {
        scale: Vec3::new(2.0, 3.0, 4.0),
        rotation: Quat::from_rotation_z(FRAC_PI_2),
        translation: Vec3::new(1.0, 2.0, 3.0),
    };
    Pose { bones: vec![first, Sqt::IDENTITY] }
}

#[test]
fn matrix_applies_the_scale_first_then_the_rotation_then_the_translation() {
    let sqt = two_bones().bones[0];
    // (1, 0, 0) scales to (2, 0, 0), turns a quarter about z to (0, 2, 0), then moves to (1, 4, 3).
    let moved = sqt.matrix().transform_point3(Vec3::X);
    assert!(moved.abs_diff_eq(Vec3::new(1.0, 4.0, 3.0), 1e-5), "{moved:?}");
    let reversed =
        Mat4::from_scale(sqt.scale) * Mat4::from_quat(sqt.rotation) * Mat4::from_translation(sqt.translation);
    assert!(!reversed.abs_diff_eq(sqt.matrix(), 1e-3), "the order must not commute");
}

#[test]
fn identity_matrix_is_the_identity() {
    assert_eq!(Sqt::IDENTITY.matrix(), Mat4::IDENTITY);
}

#[test]
fn dof_buffer_round_trips_and_leaves_the_unused_slots_alone() {
    let pose = two_bones();
    let mut buffer = vec![7.0f32; 2 * DOFS_PER_BONE];
    pose.write_dofs(&mut buffer);
    assert_eq!(buffer[0..3], [2.0, 3.0, 4.0]);
    assert_eq!(buffer[4..8], Quat::from_rotation_z(FRAC_PI_2).to_array());
    assert_eq!(buffer[8..11], [1.0, 2.0, 3.0]);
    assert_eq!(buffer[3], 7.0, "slot 3 is unused");
    assert_eq!(buffer[11], 7.0, "slot 11 is unused");
    assert_eq!(buffer[12..15], [1.0, 1.0, 1.0], "the second bone's scale");
    assert_eq!(buffer[19], 1.0, "the second bone's quaternion w");

    let back = Pose::from_dofs(&buffer);
    assert_eq!(back.bones.len(), 2);
    assert!(back.bones[0].rotation.abs_diff_eq(pose.bones[0].rotation, 1e-6));
    assert_eq!(back.bones[0].scale, pose.bones[0].scale);
    assert_eq!(back.bones[0].translation, pose.bones[0].translation);
    assert_eq!(back.bones[1], Sqt::IDENTITY);
}

#[test]
fn from_dofs_normalises_the_quaternion_and_maps_zero_to_identity() {
    let mut buffer = vec![0.0f32; 2 * DOFS_PER_BONE];
    buffer[0..3].copy_from_slice(&[1.0, 1.0, 1.0]);
    buffer[4..8].copy_from_slice(&[0.0, 0.0, 2.0, 0.0]);
    buffer[DOFS_PER_BONE + 4..DOFS_PER_BONE + 8].copy_from_slice(&[0.0, 0.0, 0.0, 0.0]);
    let pose = Pose::from_dofs(&buffer);
    assert!(pose.bones[0].rotation.abs_diff_eq(Quat::from_xyzw(0.0, 0.0, 1.0, 0.0), 1e-6));
    assert!((pose.bones[0].rotation.length() - 1.0).abs() < 1e-6);
    assert_eq!(pose.bones[1].rotation, Quat::IDENTITY);
}

#[test]
fn from_dofs_ignores_the_unused_slots() {
    let mut buffer = vec![f32::NAN; DOFS_PER_BONE];
    buffer[0..3].copy_from_slice(&[1.0, 1.0, 1.0]);
    buffer[4..8].copy_from_slice(&[0.0, 0.0, 0.0, 1.0]);
    buffer[8..11].copy_from_slice(&[0.5, 0.0, 0.0]);
    let pose = Pose::from_dofs(&buffer);
    assert_eq!(pose.bones[0].scale, Vec3::ONE);
    assert_eq!(pose.bones[0].translation, Vec3::new(0.5, 0.0, 0.0));
}

#[test]
#[should_panic(expected = "13 floats")]
fn from_dofs_rejects_a_partial_bone() {
    let _ = Pose::from_dofs(&[0.0; 13]);
}

#[test]
#[should_panic(expected = "needs a DOF buffer")]
fn write_dofs_rejects_a_buffer_of_the_wrong_size() {
    two_bones().write_dofs(&mut [0.0; DOFS_PER_BONE]);
}
