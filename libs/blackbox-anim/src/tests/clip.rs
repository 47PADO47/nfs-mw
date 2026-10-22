//! Clip tests on synthetic compounds built by `fixture`. Each child is a DeltaChan leaf written by `leaf` below, with
//! 8-bit rows of range 1 and minimum 0, so a raw byte is the step in value. No install data is read here.

use glam::{Quat, Vec3};

use super::fixture::{Data, attribute_block, bank, child, compound};
use crate::clip::{Clip, clips};
use crate::{Animation, Error, Object, Pose, Sqt};

const CHECKSUM: u16 = 0x1414;
const DELTA_LERP: u16 = 10;
const DELTA_QUAT: u16 = 11;
/// StatelessQ (type 22): a valid type id with no codec in this reader, so it is skipped.
const UNSUPPORTED: u16 = 22;
const SLOTS: usize = 24;

/// A DeltaChan leaf of `type_id` (10 or 11). Dof `dofs[i]` starts at `starts[i]` and gains the raw step `rows[f][i]`
/// from frame `f` to frame `f + 1`, so the leaf has `rows.len() + 1` frames. A quaternion leaf lists one dof and four
/// starts. Returns the leaf's offset.
fn leaf(data: &mut Data, type_id: u16, dofs: &[u16], starts: &[f32], rows: &[Vec<u8>]) -> u32 {
    data.align();
    let block = data.here();
    data.put_u16(starts.len() as u16);
    data.put_bytes(&[8, 0]);
    for &start in starts {
        data.put_bytes(&0.0f32.to_le_bytes());
        data.put_bytes(&1.0f32.to_le_bytes());
        data.put_bytes(&start.to_le_bytes());
    }
    for row in rows {
        data.put_bytes(row);
    }
    data.align();
    let at = data.here();
    data.put_u16(type_id);
    data.put_u16(CHECKSUM);
    data.put_pointer(block);
    data.put_u16(rows.len() as u16 + 1);
    for &dof in dofs {
        data.put_u16(dof);
    }
    at
}

/// The clip read from the compound at `at`, as the animation `test_q`.
fn read_clip(data: &Data, at: u32) -> Clip {
    let object = Object::read(&data.builder(None).padded()).expect("the synthetic object reads");
    let animation = Animation { name: "test_q".to_string(), type_id: 15, checksum: CHECKSUM, offset: at };
    Clip::read(&object, &animation).expect("the clip reads")
}

/// The DOF buffer after sampling `clip` at `secs`, from zeros.
fn sample_at(clip: &Clip, secs: f32) -> Vec<f32> {
    let mut out = vec![0.0; SLOTS];
    clip.sample(secs, &mut out);
    out
}

fn assert_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() < 1e-4, "{actual:?} is not {expected:?}");
    }
}

/// A three-frame compound at 30 fps (0.1 s). Child 0 is a quaternion at dof 4 (bone 0 rotation): the identity, then
/// steps of 1 along x. Child 1 is a scalar at dof 20 (bone 1 translation x): 1, 3, 7.
fn walk_clip() -> (Data, u32) {
    let mut data = Data::default();
    let quat = leaf(&mut data, DELTA_QUAT, &[4], &[0.0, 0.0, 0.0, 1.0], &[vec![1, 0, 0, 0], vec![1, 0, 0, 0]]);
    let lerp = leaf(&mut data, DELTA_LERP, &[20], &[1.0], &[vec![2], vec![4]]);
    let block = attribute_block(&mut data, &[(1, 30)]);
    let at = compound(&mut data, CHECKSUM, 3, Some(block), &[quat, lerp]);
    (data, at)
}

#[test]
fn reads_the_name_checksum_rate_and_length() {
    let (data, at) = walk_clip();
    let clip = read_clip(&data, at);
    assert_eq!(clip.name, "test_q");
    assert_eq!(clip.checksum, CHECKSUM);
    assert_eq!(clip.fps, 30.0);
    assert_eq!(clip.frames, 3);
    assert!(clip.skipped.is_empty());
    assert_eq!(clip.channels().len(), 2);
    assert!((clip.duration_secs() - 0.1).abs() < 1e-6, "3 frames at 30 fps is 0.1 s");
}

#[test]
fn samples_at_the_start_the_middle_and_the_end() {
    let (data, at) = walk_clip();
    let clip = read_clip(&data, at);
    let start = sample_at(&clip, 0.0);
    assert_close(&start[4..8], &[0.0, 0.0, 0.0, 1.0]);
    assert_close(&start[20..21], &[1.0]);
    let middle = sample_at(&clip, 0.05);
    assert_close(&middle[4..8], &[1.5, 0.0, 0.0, 1.0]);
    assert_close(&middle[20..21], &[5.0]);
    let end = sample_at(&clip, clip.duration_secs());
    assert_close(&end[4..8], &[2.0, 0.0, 0.0, 1.0]);
    assert_close(&end[20..21], &[7.0]);
}

#[test]
fn the_frame_is_clamped_to_the_clip() {
    let (data, at) = walk_clip();
    let clip = read_clip(&data, at);
    assert!((clip.frame_at(0.05) - 1.5).abs() < 1e-4);
    assert_eq!(clip.frame_at(10.0), 2.0, "past the end is the last frame");
    assert_eq!(clip.frame_at(-1.0), 0.0, "before the start is frame 0");
    assert_eq!(clip.frame_at(f32::NAN), 0.0, "a NaN time is frame 0");
}

#[test]
fn sampling_past_the_end_holds_the_last_frame() {
    let (data, at) = walk_clip();
    let clip = read_clip(&data, at);
    let late = sample_at(&clip, 10.0);
    assert_close(&late[4..8], &[2.0, 0.0, 0.0, 1.0]);
    assert_close(&late[20..21], &[7.0]);
    let early = sample_at(&clip, -4.0);
    assert_close(&early[4..8], &[0.0, 0.0, 0.0, 1.0]);
    assert_close(&early[20..21], &[1.0]);
}

#[test]
fn an_earlier_child_overwrites_a_later_one_on_the_same_slot() {
    // Both children drive dof 20. The last child is visited first, so child 0 (value 5) is the one left in the buffer.
    let mut data = Data::default();
    let first = leaf(&mut data, DELTA_LERP, &[20], &[5.0], &[vec![0], vec![0]]);
    let second = leaf(&mut data, DELTA_LERP, &[20], &[9.0], &[vec![0], vec![0]]);
    let block = attribute_block(&mut data, &[(1, 30)]);
    let at = compound(&mut data, CHECKSUM, 3, Some(block), &[first, second]);
    let clip = read_clip(&data, at);
    assert_close(&sample_at(&clip, 0.0)[20..21], &[5.0]);
}

#[test]
fn sample_pose_keeps_the_base_values_of_undriven_slots() {
    let (data, at) = walk_clip();
    let clip = read_clip(&data, at);
    let base = Pose {
        bones: vec![
            Sqt { scale: Vec3::splat(2.0), rotation: Quat::IDENTITY, translation: Vec3::new(5.0, 0.0, 0.0) },
            Sqt { scale: Vec3::ONE, rotation: Quat::IDENTITY, translation: Vec3::new(0.0, 7.0, 0.0) },
        ],
    };
    let pose = clip.sample_pose(0.05, &base);
    let expected_rotation = Quat::from_xyzw(1.5, 0.0, 0.0, 1.0).normalize().to_array();
    assert_close(&pose.bones[0].rotation.to_array(), &expected_rotation);
    assert_eq!(pose.bones[0].scale, Vec3::splat(2.0), "bone 0 scale is not driven");
    assert_eq!(pose.bones[0].translation, Vec3::new(5.0, 0.0, 0.0), "bone 0 translation is not driven");
    assert_close(&[pose.bones[1].translation.x], &[5.0]);
    assert_eq!(pose.bones[1].translation.y, 7.0, "bone 1 y is not driven");
    assert_eq!(pose.bones[1].rotation, Quat::IDENTITY, "bone 1 rotation is not driven");
}

#[test]
fn an_unsupported_child_is_skipped_and_reported() {
    let mut data = Data::default();
    let odd = child(&mut data, UNSUPPORTED, 0);
    let lerp = leaf(&mut data, DELTA_LERP, &[20], &[1.0], &[vec![2], vec![4]]);
    let block = attribute_block(&mut data, &[(1, 30)]);
    let at = compound(&mut data, CHECKSUM, 3, Some(block), &[odd, lerp]);
    let clip = read_clip(&data, at);
    assert_eq!(clip.skipped, [UNSUPPORTED]);
    assert_eq!(clip.channels().len(), 1, "the readable child is kept");
    assert_eq!(clip.max_dof(), 21);
    let end = sample_at(&clip, clip.duration_secs());
    assert_close(&end[20..21], &[7.0]);
    assert!(end[..20].iter().all(|&slot| slot == 0.0), "the skipped child writes nothing");
}

#[test]
fn max_dof_is_the_reach_of_the_widest_channel() {
    // A quaternion at dof 16 writes floats 16 to 19; a scalar at dof 22 writes float 22. The reach is 23.
    let mut data = Data::default();
    let quat = leaf(&mut data, DELTA_QUAT, &[16], &[0.0, 0.0, 0.0, 1.0], &[vec![0, 0, 0, 0]]);
    let lerp = leaf(&mut data, DELTA_LERP, &[22], &[1.0], &[vec![0]]);
    let at = compound(&mut data, CHECKSUM, 2, None, &[quat, lerp]);
    let clip = read_clip(&data, at);
    assert_eq!(clip.max_dof(), 23);
    assert_eq!(clip.max_dof().div_ceil(crate::DOFS_PER_BONE), 2, "two bones");
}

#[test]
fn a_clip_with_only_skipped_children_needs_no_bones() {
    let mut data = Data::default();
    let odd = child(&mut data, UNSUPPORTED, 0);
    let at = compound(&mut data, CHECKSUM, 2, None, &[odd]);
    let clip = read_clip(&data, at);
    assert!(clip.channels().is_empty());
    assert_eq!(clip.max_dof(), 0);
}

#[test]
fn a_corrupt_child_fails_the_clip() {
    // A DeltaLerp record with a null data block is corrupt, not unsupported.
    let mut data = Data::default();
    let broken = child(&mut data, DELTA_LERP, 0);
    let at = compound(&mut data, CHECKSUM, 2, None, &[broken]);
    let object = Object::read(&data.builder(None).padded()).expect("the synthetic object reads");
    let animation = Animation { name: "bad_q".to_string(), type_id: 15, checksum: CHECKSUM, offset: at };
    assert!(matches!(Clip::read(&object, &animation), Err(Error::Invalid(_))));
}

#[test]
fn clips_reads_every_animation_of_the_bank_in_bank_order() {
    let mut data = Data::default();
    let lerp_a = leaf(&mut data, DELTA_LERP, &[20], &[1.0], &[vec![2], vec![4]]);
    let block = attribute_block(&mut data, &[(1, 30)]);
    let first = compound(&mut data, CHECKSUM, 3, Some(block), &[lerp_a]);
    let lerp_b = leaf(&mut data, DELTA_LERP, &[0], &[0.5], &[vec![1], vec![1]]);
    let second = compound(&mut data, CHECKSUM, 3, Some(block), &[lerp_b]);
    let header = bank(&mut data, &[("a_t", first), ("b_t", second)]);
    let object = Object::read(&data.builder(Some(header)).padded()).expect("the synthetic object reads");
    let found = clips(&object).expect("the clips read");
    let names: Vec<&str> = found.iter().map(|clip| clip.name.as_str()).collect();
    assert_eq!(names, ["a_t", "b_t"]);
    assert_close(&sample_at(&found[1], 0.0)[0..1], &[0.5]);
}
