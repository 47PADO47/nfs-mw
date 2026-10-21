use glam::{Mat4, Quat, Vec3};

use super::elf_builder::Builder;
use crate::{Error, Object, Skeleton};

const RIG: &str = "rig";
const CHECKSUM: u16 = 0x1658;
const TOLERANCE: f32 = 1e-5;

/// One bone as its record stores it. The names stand in for the bone names of real rigs.
#[derive(Clone, Copy)]
struct TestBone {
    name: &'static str,
    parent: i32,
    scale: Vec3,
    rotation: Quat,
    translation: Vec3,
}

/// A three-bone chain with a scale, a rotation and a translation on every bone.
fn chain() -> Vec<TestBone> {
    vec![
        TestBone {
            name: RIG,
            parent: -1,
            scale: Vec3::ONE,
            rotation: Quat::from_rotation_z(0.5),
            translation: Vec3::new(0.0, 0.0, 1.0),
        },
        TestBone {
            name: "arm",
            parent: 0,
            scale: Vec3::new(1.0, 1.0, 2.0),
            rotation: Quat::from_rotation_x(0.3),
            translation: Vec3::new(0.5, 0.0, 0.0),
        },
        TestBone {
            name: "hand",
            parent: 1,
            scale: Vec3::new(0.5, 1.0, 1.0),
            rotation: Quat::from_rotation_y(-0.7),
            translation: Vec3::new(0.0, 0.25, 0.0),
        },
    ]
}

/// The bind world matrix of every bone, from the formula `T * R * S` written out directly.
fn bind_worlds(bones: &[TestBone]) -> Vec<Mat4> {
    let mut worlds: Vec<Mat4> = Vec::new();
    for bone in bones {
        let local =
            Mat4::from_translation(bone.translation) * Mat4::from_quat(bone.rotation) * Mat4::from_scale(bone.scale);
        let world = match bone.parent {
            -1 => local,
            parent => worlds[parent as usize] * local,
        };
        worlds.push(world);
    }
    worlds
}

fn push_f32s(out: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        out.extend(value.to_le_bytes());
    }
}

/// A 112-byte bone record: scale, parent, quaternion x y z w, translation, left/right field, inverse bind.
fn record(bone: &TestBone, index: usize, inverse: &Mat4) -> Vec<u8> {
    let mut out = Vec::with_capacity(112);
    push_f32s(&mut out, &bone.scale.to_array());
    out.extend(bone.parent.to_le_bytes());
    push_f32s(&mut out, &bone.rotation.to_array());
    push_f32s(&mut out, &bone.translation.to_array());
    out.extend((index as i32).to_le_bytes());
    push_f32s(&mut out, &inverse.to_cols_array());
    assert_eq!(out.len(), 112);
    out
}

/// The `.data` of a rig: one index record per bone, the 16-byte header, then the bone records.
fn rig_data(bones: &[TestBone], inverses: &[Mat4]) -> Vec<u8> {
    let count = bones.len();
    let mut data = Vec::new();
    for index in 0..count {
        data.extend((index as u32).to_le_bytes());
        data.extend([0u8; 12]);
    }
    data.extend(CHECKSUM.to_le_bytes());
    data.extend(((10 * count) as u16).to_le_bytes());
    data.extend([0u8; 4]); // core checksum and flags
    data.extend((count as i32).to_le_bytes());
    data.extend([0u8; 4]); // pointer to the inverse bone scales
    for (index, (bone, inverse)) in bones.iter().zip(inverses).enumerate() {
        data.extend(record(bone, index, inverse));
    }
    data
}

/// The symbols of a rig whose `.data` is `data`: `__Skeleton` at the header and one `__Bone` per index record.
fn rig_builder(data: &[u8], bones: &[TestBone]) -> Builder {
    let mut builder = Builder::new(data).symbol(&format!("__Skeleton:::{RIG}"), (16 * bones.len()) as u32);
    for (index, bone) in bones.iter().enumerate() {
        builder = builder.symbol(&format!("__Bone:::{RIG}.{}", bone.name), (16 * index) as u32);
    }
    builder
}

/// The chain with inverse bind matrices that are the true inverses of its bind world matrices.
fn chain_object() -> Object {
    let bones = chain();
    let inverses: Vec<Mat4> = bind_worlds(&bones).iter().map(|world| world.inverse()).collect();
    let data = rig_data(&bones, &inverses);
    Object::read(&rig_builder(&data, &bones).padded()).unwrap()
}

fn assert_near(actual: Mat4, expected: Mat4) {
    assert!(actual.abs_diff_eq(expected, TOLERANCE), "{actual:?} is not {expected:?}");
}

#[test]
fn reads_names_parents_and_bind_values() {
    let skeleton = Skeleton::read(&chain_object()).unwrap();
    assert_eq!(skeleton.name, RIG);
    assert_eq!(skeleton.checksum, CHECKSUM);
    let names: Vec<&str> = skeleton.bones.iter().map(|bone| bone.name.as_str()).collect();
    assert_eq!(names, [RIG, "arm", "hand"]);
    let parents: Vec<Option<usize>> = skeleton.bones.iter().map(|bone| bone.parent).collect();
    assert_eq!(parents, [None, Some(0), Some(1)]);
    for (bone, expected) in skeleton.bones.iter().zip(chain()) {
        assert_eq!(bone.bind.scale, expected.scale);
        assert_eq!(bone.bind.rotation, expected.rotation);
        assert_eq!(bone.bind.translation, expected.translation);
    }
}

#[test]
fn bone_index_finds_a_bone_by_name() {
    let skeleton = Skeleton::read(&chain_object()).unwrap();
    assert_eq!(skeleton.bone_index("arm"), Some(1));
    assert_eq!(skeleton.bone_index("hand"), Some(2));
    assert_eq!(skeleton.bone_index("leg"), None);
}

#[test]
fn bind_pose_world_matrices_chain_the_parents() {
    let skeleton = Skeleton::read(&chain_object()).unwrap();
    let world = skeleton.world(&skeleton.bind_pose());
    let expected = bind_worlds(&chain());
    assert_eq!(world.len(), 3);
    for (actual, expected) in world.into_iter().zip(expected) {
        assert_near(actual, expected);
    }
}

#[test]
fn skinning_of_the_bind_pose_is_the_identity() {
    let skeleton = Skeleton::read(&chain_object()).unwrap();
    for skin in skeleton.skinning(&skeleton.bind_pose()) {
        assert_near(skin, Mat4::IDENTITY);
    }
}

#[test]
fn a_moved_bone_moves_its_children_and_not_its_parents() {
    let skeleton = Skeleton::read(&chain_object()).unwrap();
    let mut pose = skeleton.bind_pose();
    pose.bones[1].translation += Vec3::Z;
    let arm = chain()[1];
    let moved_arm =
        Mat4::from_translation(arm.translation + Vec3::Z) * Mat4::from_quat(arm.rotation) * Mat4::from_scale(arm.scale);
    let root_world = bind_worlds(&chain())[0];
    let world = skeleton.world(&pose);
    assert_near(world[1], root_world * moved_arm);

    let skins = skeleton.skinning(&pose);
    assert_near(skins[0], Mat4::IDENTITY);
    assert!(!skins[1].abs_diff_eq(Mat4::IDENTITY, 1e-3), "the moved arm must not skin to the identity");
}

#[test]
#[should_panic(expected = "different bone counts")]
fn world_rejects_a_pose_of_another_size() {
    let skeleton = Skeleton::read(&chain_object()).unwrap();
    let _ = skeleton.world(&crate::Pose { bones: Vec::new() });
}

fn read_with_parent(index: usize, parent: i32) -> Result<Skeleton, Error> {
    let mut bones = chain();
    bones[index].parent = parent;
    let inverses = vec![Mat4::IDENTITY; bones.len()];
    let data = rig_data(&bones, &inverses);
    Skeleton::read(&Object::read(&rig_builder(&data, &bones).padded()).unwrap())
}

#[test]
fn a_parent_must_be_an_earlier_bone_or_none() {
    assert!(read_with_parent(1, 0).is_ok());
    assert!(read_with_parent(1, -1).is_ok(), "a second root is allowed");
    assert!(matches!(read_with_parent(1, 2), Err(Error::Invalid(_))), "a later parent");
    assert!(matches!(read_with_parent(2, 2), Err(Error::Invalid(_))), "a bone as its own parent");
    assert!(matches!(read_with_parent(2, -2), Err(Error::Invalid(_))), "a parent below -1");
}

#[test]
fn a_truncated_bone_record_is_rejected() {
    let bones = chain();
    let inverses = vec![Mat4::IDENTITY; bones.len()];
    let mut data = rig_data(&bones, &inverses);
    data.truncate(data.len() - 10);
    let object = Object::read(&rig_builder(&data, &bones).padded()).unwrap();
    assert!(matches!(Skeleton::read(&object), Err(Error::Truncated { what: "bone record", .. })));
}

#[test]
fn a_truncated_header_is_rejected() {
    let object = Object::read(&Builder::new(&[0u8; 16]).symbol("__Skeleton:::rig", 16).padded()).unwrap();
    assert_eq!(Skeleton::read(&object), Err(Error::Truncated { what: "skeleton header", offset: 16 }));
}

#[test]
fn an_object_without_a_skeleton_symbol_is_rejected() {
    let object = Object::read(&Builder::new(&[0u8; 16]).padded()).unwrap();
    assert!(matches!(Skeleton::read(&object), Err(Error::Invalid(_))));
}

#[test]
fn a_bone_of_another_rig_is_rejected() {
    let bones = chain();
    let inverses = vec![Mat4::IDENTITY; bones.len()];
    let data = rig_data(&bones, &inverses);
    let builder = rig_builder(&data, &bones).symbol("__Bone:::other.arm", 0);
    let object = Object::read(&builder.padded()).unwrap();
    assert!(matches!(Skeleton::read(&object), Err(Error::Invalid(_))));
}

#[test]
fn an_index_record_must_name_a_bone_of_the_rig() {
    let bones = chain();
    let inverses = vec![Mat4::IDENTITY; bones.len()];
    let mut data = rig_data(&bones, &inverses);
    data[0..4].copy_from_slice(&9u32.to_le_bytes());
    let object = Object::read(&rig_builder(&data, &bones).padded()).unwrap();
    assert!(matches!(Skeleton::read(&object), Err(Error::Invalid(_))), "index 9 of 3 bones");
}

#[test]
fn two_bones_with_one_index_are_rejected() {
    let bones = chain();
    let inverses = vec![Mat4::IDENTITY; bones.len()];
    let mut data = rig_data(&bones, &inverses);
    data[16..20].copy_from_slice(&0u32.to_le_bytes());
    let object = Object::read(&rig_builder(&data, &bones).padded()).unwrap();
    assert!(matches!(Skeleton::read(&object), Err(Error::Invalid(_))), "index 0 used twice, index 1 missing");
}
