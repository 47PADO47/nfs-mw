//! The fallen rider stays in one piece: the helmet on the head, the head on the neck, through the fall, the get-up and the way back
//! onto the bike (`fall_pose.rs`; `docs/specs/ride6-rider-fall.md`, "One piece").

use blackbox_skeleton::{Bone, Skeleton, Transform};
use blackbox_vehicle::bike::{FallKind, RiderRelease};
use blackbox_vehicle::{FIXED_STEP, FlatGround};
use glam::{Mat4, Quat, Vec3};

use super::fall_file::RagdollFile;
use super::fall_pose::{blend_back, hung};
use super::*;

/// The most a bone may be off its parent's frame, metres: a millimetre, the rounding of the maths and the file.
const TOLERANCE: f32 = 0.001;
const SPINE: usize = 2;
const NECK: usize = 3;
const HEAD: usize = 4;
const HELMET: usize = 5;
/// The helmet's offset from the head bone: the real rider's is 1.5 cm; this one is bigger so that a shift shows.
const HELMET_OFFSET: Vec3 = Vec3::new(0.0, 0.02, 0.03);

/// A rider reduced to what the tear is about: a pelvis, a spine, a neck that has no body, a head and a helmet on it.
fn skeleton() -> Skeleton {
    let bone = |name: &str, parent: Option<usize>, t: Vec3| Bone {
        name: name.into(),
        parent,
        rest: Transform::from_rt(Quat::IDENTITY, t),
    };
    Skeleton::new(vec![
        bone("root", None, Vec3::ZERO),
        bone("pelvis", Some(0), Vec3::new(0.0, 0.0, 1.0)),
        bone("spine_02", Some(1), Vec3::new(0.0, 0.0, 0.25)),
        bone("neck_01", Some(2), Vec3::new(0.0, 0.0, 0.3)),
        bone("head", Some(3), Vec3::new(0.0, 0.0, 0.1)),
        bone("helmet", Some(4), HELMET_OFFSET),
    ])
    .unwrap()
}

/// The pelvis, the spine and the head have bodies; the neck and the helmet have none. The head's joint is as free as the real one
/// (180 degrees each way) so that it can be thrown about.
const FILE: &str = r#"{
    "bodies": [
        {"bone": "pelvis", "mass": 7.0, "radius": 0.11, "half_length": 0.075, "centre": [0, 0, 0], "axis": [0, 1, 0]},
        {"bone": "spine_02", "mass": 20.0, "radius": 0.19, "half_length": 0.1, "centre": [0, 0, 0.1], "axis": [0, 1, 0]},
        {"bone": "head", "mass": 4.3, "radius": 0.13, "half_length": 0.05, "centre": [0, 0, 0.1], "axis": [0, 0, 1]}
    ],
    "joints": [
        {"child": "spine_02", "parent": "pelvis", "frame_child": [0, 0, 0, 1], "frame_parent": [0, 0, 0, 1],
         "swing_z_deg": 40, "swing_y_deg": 40, "twist_deg": 30},
        {"child": "head", "parent": "spine_02", "frame_child": [0, 0, 0, 1], "frame_parent": [0, 0, 0, 1],
         "swing_z_deg": 180, "swing_y_deg": 180, "twist_deg": 180}
    ],
    "contact_friction": 0.66,
    "joint_damping": 4.0,
    "falls": {"velocity_multiplier": 1.0,
              "high_side": {"additional": [0, 0, 1.6667], "scale": [0.05, 0, -0.28], "world": [0, 0, 0], "angular": 0.5},
              "low_side": {"additional": [0, 0, 0], "scale": [0.05, 0, 0.32], "world": [0, 0, 0], "angular": 0.5}}
}"#;

fn release() -> RiderRelease {
    RiderRelease {
        kind: FallKind::HighSide,
        position: Vec3::new(0.0, 1.2, 0.0),
        velocity: Vec3::new(0.0, 0.0, 22.0),
        angular_velocity: Vec3::new(0.0, 0.0, 6.0),
        orientation: Quat::IDENTITY,
    }
}

fn seated() -> Mat4 {
    Mat4::from_translation(Vec3::new(0.0, 0.0, 0.2))
}

fn riding(skeleton: &Skeleton) -> Vec<Transform> {
    let mut out = Vec::new();
    skeleton.component_transforms(&skeleton.rest_pose().local, &mut out).unwrap();
    out
}

/// How far the bones under the pelvis (the free body) are from where their parent's frame puts them, and how far the helmet has turned against the head.
fn tear(skeleton: &Skeleton, pose: &[Transform]) -> (f32, f32) {
    let mut worst = 0.0_f32;
    for (i, bone) in skeleton.bones().iter().enumerate().skip(SPINE) {
        let Some(parent) = bone.parent else { continue };
        let want = pose[parent].compose(skeleton.rest_pose().local[i]).translation;
        worst = worst.max(pose[i].translation.distance(want));
    }
    let turned = (pose[HEAD].rotation.inverse() * pose[HELMET].rotation).angle_between(Quat::IDENTITY);
    (worst, turned)
}

fn started() -> (Fall, Skeleton) {
    let skeleton = skeleton();
    let mut fall = Fall::new(Some(RagdollFile::parse(FILE.as_bytes()).unwrap()));
    fall.remember(&riding(&skeleton));
    assert!(fall.start(&skeleton, (seated(), Quat::IDENTITY), &release()));
    (fall, skeleton)
}

/// A recorded fall: a high side at 79 km/h, 6 s of it at the physics' fixed step, the pose taken every 4th step.
#[test]
fn the_helmet_stays_on_the_head_and_the_head_on_the_neck_through_a_whole_fall() {
    let (mut fall, skeleton) = started();
    let ground = FlatGround::new(0.0);
    let mut worst = (0.0_f32, 0.0_f32);
    let mut head_moved = 0.0_f32;
    for step in 0..(6.0 / FIXED_STEP) as usize {
        fall.step(FIXED_STEP, &ground);
        if step % 4 != 0 {
            continue;
        }
        let pose = fall.pose(&skeleton, &riding(&skeleton), seated()).expect("down");
        let (off, turned) = tear(&skeleton, &pose);
        worst = (worst.0.max(off), worst.1.max(turned));
        let against_spine = (pose[HEAD].rotation.inverse() * pose[SPINE].rotation).angle_between(Quat::IDENTITY);
        head_moved = head_moved.max(against_spine);
    }
    assert!(head_moved > 0.3, "the fall threw his head about ({head_moved} rad), or this tests nothing");
    assert!(worst.0 < TOLERANCE, "a bone left its parent's frame by {} m", worst.0);
    assert!(worst.1 < 1e-3, "the helmet turned {} rad against the head", worst.1);
}

#[test]
fn the_helmet_stays_on_the_head_while_he_gets_back_on_the_bike_and_after_a_reset_by_hand() {
    for ready in [true, false] {
        let (mut fall, skeleton) = started();
        let ground = FlatGround::new(0.0);
        for _ in 0..(3.0 / FIXED_STEP) as usize {
            fall.step(FIXED_STEP, &ground);
        }
        let sitting = riding(&skeleton);
        // A rider who got up (the bike was stood up for him) takes a walk's time; a reset by hand is the short blend.
        fall.ready = ready;
        fall.stop(&skeleton, Vec3::new(0.0, 0.0, 0.2));
        let mut frames = 0;
        while fall.active() {
            let pose = fall.pose(&skeleton, &sitting, seated()).expect("returning");
            let (off, turned) = tear(&skeleton, &pose);
            assert!(off < TOLERANCE, "ready {ready}, frame {frames}: a bone is {off} m off its parent's frame");
            assert!(turned < 1e-3, "ready {ready}, frame {frames}: the helmet turned {turned} rad against the head");
            fall.step(FIXED_STEP, &ground);
            frames += 1;
            assert!(frames < 1000, "the way back never ended");
        }
    }
}

#[test]
fn a_bone_with_a_body_is_drawn_at_the_place_its_parent_gives_it_and_with_the_bodys_turn() {
    let turn = Quat::from_rotation_y(1.0);
    let body = Transform::from_rt(turn, Vec3::new(5.0, 0.0, 0.0));
    let parent = Transform::from_rt(Quat::from_rotation_z(0.5), Vec3::new(1.0, 2.0, 3.0));
    let local = Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.1));
    let joined = hung(body, Some(parent), local);
    assert!(joined.translation.distance(parent.compose(local).translation) < 1e-6, "the parent's place");
    assert_eq!(joined.rotation, turn, "the body's turn");
    assert_eq!(hung(body, None, local), body, "a bone with no parent is where its body is");
    let nan = Transform::from_rt(Quat::IDENTITY, Vec3::NAN);
    assert_eq!(hung(body, Some(nan), local), body, "a broken parent does not take the bone with it");
}

#[test]
fn the_way_back_keeps_every_bone_its_length_where_blending_each_bone_alone_did_not() {
    let skeleton = skeleton();
    let sitting = riding(&skeleton);
    // He lies on his back: the whole rider turned a half turn about the lateral axis and put down 3 m away.
    let flip = Quat::from_rotation_y(2.4);
    let lying: Vec<Transform> = sitting
        .iter()
        .map(|t| Transform::from_rt(flip * t.rotation, flip * t.translation + Vec3::new(3.0, 1.0, 0.1)))
        .collect();
    let (turn, seat) = (Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.2));
    for k in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let pose = blend_back(&skeleton, &lying, &sitting, (turn, seat), k);
        let (off, turned) = tear(&skeleton, &pose);
        assert!(off < 1e-4, "k {k}: {off}");
        assert!(turned < 1e-4, "k {k}: {turned}");
    }
    // The ends are the two poses: where he lay, and on the bike.
    let start = blend_back(&skeleton, &lying, &sitting, (turn, seat), 0.0);
    let end = blend_back(&skeleton, &lying, &sitting, (turn, seat), 1.0);
    for bone in 0..skeleton.len() {
        assert!((start[bone].translation + seat).distance(lying[bone].translation) < 1e-4, "start, bone {bone}");
        assert!(end[bone].translation.distance(sitting[bone].translation) < 1e-4, "end, bone {bone}");
    }
    // Per-bone blending of the same two poses, as it was done: the head's distance from the neck is not its length half way.
    let per_bone = |k: f32| {
        let on = |b: &Transform| Transform::from_rt(turn * b.rotation, turn * b.translation + seat);
        let (head, neck) = (lying[HEAD].lerp(on(&sitting[HEAD]), k), lying[NECK].lerp(on(&sitting[NECK]), k));
        head.translation.distance(neck.translation)
    };
    assert!((per_bone(0.5) - 0.1).abs() > 0.01, "the old blend tore the neck: {}", per_bone(0.5));
}
