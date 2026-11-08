use super::super::attach::tests::SAMPLE;
use super::*;
use blackbox_skeleton::{Bone, Node, TimeMode};
use glam::Quat;

/// A rider reduced to what the file names: a root, a pelvis, one arm (upperarm_l > lowerarm_l > hand_l, each 0.3 m up)
/// and its pole bone.
fn skeleton() -> Skeleton {
    let up = Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.3));
    let bone = |name: &str, parent: Option<usize>, rest: Transform| Bone { name: name.into(), parent, rest };
    Skeleton::new(vec![
        bone("root", None, Transform::IDENTITY),
        bone("pelvis", Some(0), Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.1))),
        bone("upperarm_l", Some(1), up),
        bone("lowerarm_l", Some(2), up),
        bone("hand_l", Some(3), up),
        bone("PV_arm_l", Some(0), Transform::from_rt(Quat::IDENTITY, Vec3::new(0.5, 0.0, 0.3))),
    ])
    .unwrap()
}

pub(super) fn graph() -> (Graph, Vec<Clip>) {
    let clip = Clip {
        name: "rest".into(),
        length: 1.0,
        tracks: Vec::new(),
        additive: false,
        root_motion: false,
        notifies: Vec::new(),
    };
    (Graph { param_names: Vec::new(), root: Node::Clip { clip: 0, time: TimeMode::Hold } }, vec![clip])
}

fn attach() -> Attach {
    Attach::parse(SAMPLE.as_bytes()).unwrap()
}

fn frame() -> Frame {
    Frame::new(&attach().rider_root_in_bike.transform(), Vec3::new(0.0016, 0.0, 0.0797))
}

/// The car's own frame in the rider's space for the tests: the identity.
fn plain_frame() -> Frame {
    Frame::new(&Transform::IDENTITY, Vec3::ZERO)
}

fn at(position: Vec3) -> Placed {
    Placed { position, rotation: None, anchor: None }
}

/// The pose of `skeleton` in the plain frame with the given targets and body.
fn run(skeleton: &Skeleton, resolved: &Resolved, targets: &[Placed], body: Body) -> Vec<Transform> {
    let (graph, clips) = graph();
    let frame = plain_frame();
    let goals = Goals { targets, body, frame: &frame, release: [0.0; 2] };
    animate(skeleton, &clips, &graph, (&[], 0.0), resolved, &goals).unwrap()
}

#[test]
fn the_names_of_the_file_are_found_in_the_skeleton_or_the_missing_one_is_named() {
    let resolved = Resolved::new(&attach(), &skeleton()).unwrap();
    assert_eq!(resolved.limbs.len(), 1);
    assert_eq!(resolved.limbs[0].chain, TwoBoneChain { root: 2, mid: 3, tip: 4 });
    assert_eq!(resolved.limbs[0].pole, 5);
    assert_eq!(resolved.limbs[0].target, 0);
    assert_eq!(resolved.shift_bone, 1);
    assert!(resolved.limbs[0].frame.is_some(), "the file's tip frame is read");
    assert!(resolved.torso.is_none() && resolved.seat.is_none());
    let mut broken = attach();
    broken.limbs[0].tip = "claw".into();
    assert!(Resolved::new(&broken, &skeleton()).unwrap_err().contains("claw"));
    let mut lost = attach();
    lost.limbs[0].target = "elsewhere".into();
    assert!(Resolved::new(&lost, &skeleton()).unwrap_err().contains("elsewhere"));
}

#[test]
fn the_frames_agree_and_undo_each_other() {
    let frame = frame();
    let point = Vec3::new(0.3, -0.2, 0.5);
    assert!(frame.to_rider.transform_point3(frame.to_model.transform_point3(point)).distance(point) < 1e-5);
    // The seat's quarter turn about z, then the move: the rider's origin is at the seat plus the offset.
    let origin = frame.to_model.transform_point3(Vec3::ZERO);
    assert!(origin.distance(Vec3::new(-0.34 + 0.0016, 0.0, 0.91 + 0.0797)) < 1e-5, "{origin:?}");
    // The bike skeleton's frame has no offset.
    assert!(frame.to_bike.transform_point3(Vec3::ZERO).distance(Vec3::new(-0.34, 0.0, 0.91)) < 1e-5);
    // The car's left is the rider's +x (he faces +y after the quarter turn), and up is up.
    assert!(frame.lean_axis().distance(Vec3::X) < 1e-5 || frame.lean_axis().distance(-Vec3::X) < 1e-5);
    assert!(frame.up().distance(Vec3::Z) < 1e-5);
}

#[test]
fn a_grip_turns_with_the_handlebar_about_the_steering_axis_and_a_peg_does_not() {
    let attach = attach();
    let frame = frame();
    let axis = Vec3::new(-0.4384, 0.0, 0.8988).normalize();
    let steering = Steering { axis, point: Vec3::new(0.84, 0.0, 0.0), angle: 0.3 };
    let straight = rider_targets(&attach, &frame, None);
    let turned = rider_targets(&attach, &frame, Some(steering));
    assert!(straight[1].position.distance(turned[1].position) < 1e-6, "the peg stays");
    assert!(straight[0].position.distance(turned[0].position) > 0.02, "the grip moves");
    // The grip stays as far from the axis as it was.
    let in_model = |p: Vec3| frame.to_model.transform_point3(p);
    let distance = |p: Vec3| (p - steering.point).cross(axis).length();
    assert!((distance(in_model(straight[0].position)) - distance(in_model(turned[0].position))).abs() < 1e-4);
    // The grip is behind and above the axis point, so a turn to the left (positive) takes it to the right.
    let moved = in_model(turned[0].position) - in_model(straight[0].position);
    assert!(moved.y < 0.0, "{moved:?}");
    let identity = Steering { angle: 0.0, ..steering };
    assert!(rider_targets(&attach, &frame, Some(identity))[0].position.distance(straight[0].position) < 1e-6);
}

#[test]
fn a_target_with_a_turn_carries_it_into_the_riders_space_and_the_handlebar_turns_it_too() {
    let mut attach = attach();
    // The grip's frame is turned a quarter about the car's x axis; the peg's has no turn.
    attach.targets[0].rotation = Some(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2).to_array());
    let frame = frame();
    let straight = rider_targets(&attach, &frame, None);
    assert!(straight[1].rotation.is_none());
    let turn = straight[0].rotation.unwrap();
    // Taken back to the car's axes, it is the quarter turn about x.
    let in_car = frame.to_rider.to_scale_rotation_translation().1.inverse() * turn;
    assert!(in_car.angle_between(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)) < 1e-5);
    let steering = Steering { axis: Vec3::Z, point: Vec3::ZERO, angle: 0.4 };
    let steered = rider_targets(&attach, &frame, Some(steering));
    let expected = Quat::from_rotation_z(0.4) * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let in_car = frame.to_rider.to_scale_rotation_translation().1.inverse() * steered[0].rotation.unwrap();
    assert!(in_car.angle_between(expected) < 1e-5);
}

#[test]
fn the_hand_is_brought_to_its_target_plus_the_authored_offset() {
    let skeleton = skeleton();
    let resolved = Resolved::new(&attach(), &skeleton).unwrap();
    // Reachable: the shoulder is 0.4 m above the root's origin and the arm 0.6 m long.
    let target = Vec3::new(0.2, 0.1, 0.8);
    let mut limb = resolved.limbs[0];
    limb.frame = None;
    let wanted = target + limb.offset;
    let resolved = Resolved { limbs: vec![limb], ..resolved };
    let component = run(&skeleton, &resolved, &[at(target)], Body::REST);
    assert!(component[4].translation.distance(wanted) < 1e-3, "{:?} vs {wanted:?}", component[4].translation);
    // The bones keep their lengths.
    assert!((component[3].translation.distance(component[2].translation) - 0.3).abs() < 1e-3);
    assert!((component[4].translation.distance(component[3].translation) - 0.3).abs() < 1e-3);
}

#[test]
fn a_hand_with_a_frame_takes_the_targets_place_and_turn_composed_with_it() {
    let skeleton = skeleton();
    let resolved = Resolved::new(&attach(), &skeleton).unwrap();
    // The hand sits 2 cm along the target's own x, and turned 30 degrees about its z, against the target.
    let frame = Transform::from_rt(Quat::from_rotation_z(0.5), Vec3::new(0.02, 0.0, 0.0));
    let mut limb = resolved.limbs[0];
    limb.frame = Some(frame);
    let resolved = Resolved { limbs: vec![limb], ..resolved };
    let target =
        Placed { position: Vec3::new(0.2, 0.1, 0.8), rotation: Some(Quat::from_rotation_y(0.7)), anchor: None };
    let component = run(&skeleton, &resolved, &[target], Body::REST);
    let (position, turn) = tip_goal(&limb, &target);
    let want_position = target.position + Quat::from_rotation_y(0.7) * Vec3::new(0.02, 0.0, 0.0);
    assert!(position.distance(want_position) < 1e-6);
    assert!(component[4].translation.distance(position) < 1e-3);
    assert!(component[4].rotation.angle_between(turn.unwrap()) < 1e-4, "the hand takes the grip's turn");
}

#[test]
fn a_target_out_of_reach_stretches_the_arm_towards_it_without_breaking_it() {
    let skeleton = skeleton();
    let resolved = Resolved::new(&attach(), &skeleton).unwrap();
    let component = run(&skeleton, &resolved, &[at(Vec3::new(3.0, 0.0, 0.4))], Body::REST);
    let shoulder = component[2].translation;
    let hand = component[4].translation;
    assert!(hand.distance(shoulder) <= 0.6 + 1e-3);
    assert!(hand.x > shoulder.x + 0.4, "it reaches towards the target: {hand:?}");
}

#[test]
fn the_shift_moves_the_pelvis_and_what_hangs_on_it_but_not_the_pole() {
    let skeleton = skeleton();
    let mut attach = attach();
    attach.limbs.clear();
    let resolved = Resolved::new(&attach, &skeleton).unwrap();
    let rest = run(&skeleton, &resolved, &[], Body::REST);
    let shifted = run(&skeleton, &resolved, &[], Body { shift: Vec3::new(0.0, -0.05, 0.0), ..Body::REST });
    assert!(shifted[1].translation.distance(rest[1].translation + Vec3::new(0.0, -0.05, 0.0)) < 1e-6);
    assert!(shifted[4].translation.distance(rest[4].translation + Vec3::new(0.0, -0.05, 0.0)) < 1e-6);
    assert!(shifted[5].translation.distance(rest[5].translation) < 1e-6, "the pole is on the root");
}

#[test]
fn a_target_the_file_did_not_place_leaves_the_limb_alone() {
    let skeleton = skeleton();
    let resolved = Resolved::new(&attach(), &skeleton).unwrap();
    let none = run(&skeleton, &resolved, &[], Body::REST);
    let mut at_rest = Vec::new();
    skeleton.component_transforms(&skeleton.rest_pose().local, &mut at_rest).unwrap();
    assert!(none[4].translation.distance(at_rest[4].translation) < 1e-6);
}

/// A rider whose root is `pelvis` > `spine` and one bone far out to the left (+y of the car's frame).
fn arm_skeleton() -> Skeleton {
    let bone = |name: &str, parent: Option<usize>, t: Vec3| Bone {
        name: name.into(),
        parent,
        rest: Transform::from_rt(Quat::IDENTITY, t),
    };
    Skeleton::new(vec![
        bone("root", None, Vec3::ZERO),
        bone("pelvis", Some(0), Vec3::new(0.0, 0.0, 0.1)),
        bone("hand_l", Some(1), Vec3::new(0.0, 0.5, 0.0)),
        bone("hand_r", Some(1), Vec3::new(0.0, -0.5, 0.0)),
        bone("head", Some(1), Vec3::new(0.0, 0.0, 0.6)),
    ])
    .unwrap()
}

fn arm_pose(body: Body) -> Vec<Transform> {
    let resolved = Resolved { limbs: Vec::new(), shift_bone: 1, torso: None, seat: None };
    run(&arm_skeleton(), &resolved, &[], body)
}

#[test]
fn hanging_off_to_the_inside_moves_the_body_and_its_bones_that_way_and_mirrors() {
    let rest = arm_pose(Body::REST);
    // 10 cm to the right (car axes: y is left, so -y), and the same to the left.
    let right = arm_pose(body(&plain_frame(), Vec3::new(0.0, -0.1, 0.0), Vec3::ZERO));
    let left = arm_pose(body(&plain_frame(), Vec3::new(0.0, 0.1, 0.0), Vec3::ZERO));
    for bone in [1, 2, 3, 4] {
        let moved = right[bone].translation - rest[bone].translation;
        assert!(moved.distance(Vec3::new(0.0, -0.1, 0.0)) < 1e-6, "bone {bone}: {moved:?}");
        let mirrored = left[bone].translation - rest[bone].translation;
        assert!((moved + mirrored).length() < 1e-6, "mirror image");
    }
    assert_eq!(right[0].translation, rest[0].translation, "the root stays on the seat");
}

#[test]
fn the_body_turned_to_the_right_swings_the_head_to_the_right_about_the_pelvis() {
    let rest = arm_pose(Body::REST);
    let right = arm_pose(body(&plain_frame(), Vec3::ZERO, Vec3::new(0.2, 0.0, 0.0)));
    let left = arm_pose(body(&plain_frame(), Vec3::ZERO, Vec3::new(-0.2, 0.0, 0.0)));
    // The pelvis stays where it is; the head (0.6 m above it) moves toward -y (right) by about 0.6 sin(0.2).
    assert!(right[1].translation.distance(rest[1].translation) < 1e-6);
    let swing = right[4].translation - rest[4].translation;
    assert!(swing.y < -0.1 && (swing.y + 0.6 * 0.2_f32.sin()).abs() < 1e-5, "{swing:?}");
    let other = left[4].translation - rest[4].translation;
    assert!((swing.y + other.y).abs() < 1e-6 && (swing.z - other.z).abs() < 1e-6, "mirror image");
    // The arms swing about the pelvis too: the left hand rises, the right one drops, when the body rolls to the right.
    assert!(right[2].translation.z > rest[2].translation.z && right[3].translation.z < rest[3].translation.z);
}

#[test]
fn a_pitch_or_a_yaw_turns_about_the_axes_of_the_cars_frame() {
    let rest = arm_pose(Body::REST);
    // Nose up: the head goes back (-x of the car).
    let up = arm_pose(body(&plain_frame(), Vec3::ZERO, Vec3::new(0.0, 0.2, 0.0)));
    assert!(up[4].translation.x < rest[4].translation.x - 0.05, "{:?}", up[4].translation);
    // Yaw to the right: the left hand (+y) moves forward and the right hand back.
    let yaw = arm_pose(body(&plain_frame(), Vec3::ZERO, Vec3::new(0.0, 0.0, 0.2)));
    assert!(yaw[2].translation.x > rest[2].translation.x + 0.05 && yaw[3].translation.x < rest[3].translation.x - 0.05);
    // A body that is not moved or turned is the rest pose.
    assert_eq!(body(&plain_frame(), Vec3::ZERO, Vec3::ZERO).rotation, Quat::IDENTITY);
    assert_eq!(body(&plain_frame(), Vec3::ZERO, Vec3::splat(f32::NAN)).rotation, Quat::IDENTITY);
}

#[test]
fn the_throttle_rolls_the_right_grips_target_about_its_own_frame_and_only_that_one() {
    use super::super::attach::{Rigid, Target};
    let mut attach = attach();
    let turn = Quat::from_rotation_z(0.5);
    attach.targets.push(Target {
        name: "right_grip".into(),
        position: [0.12, -0.33, 1.14],
        rotation: Some(turn.to_array()),
        steers: true,
        anchor: None,
        anchor_rotation: None,
        grip_span: None,
    });
    let roll = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_6);
    attach.wrist_throttle = Some(Rigid { translation: [-0.006, -0.05, 0.0], rotation: roll.to_array() });
    let frame = frame();
    let rest = rider_targets(&attach, &frame, None);
    let mut targets = rest.clone();
    roll_wrist(&attach, &mut targets, 0.0);
    assert_eq!(targets, rest, "a closed throttle changes nothing");
    roll_wrist(&attach, &mut targets, 1.0);
    assert_eq!(targets[0], rest[0], "the left grip is not the throttle's");
    let (before, after) = (rest[2], targets[2]);
    // The offset's translation is not played: the hand would leave the grip (a user's playtest, spec section 2).
    assert_eq!(after.position, before.position, "the grip does not move");
    let rolled = before.rotation.unwrap().inverse() * after.rotation.unwrap();
    assert!(rolled.angle_between(roll) < 1e-5);
    // Half the throttle is half the roll.
    let mut half = rest.clone();
    roll_wrist(&attach, &mut half, 0.5);
    let share = before.rotation.unwrap().inverse() * half[2].rotation.unwrap();
    assert!((share.angle_between(Quat::IDENTITY) - roll.angle_between(Quat::IDENTITY) / 2.0).abs() < 1e-4);
    // A file without the offset, or a grip without a turn, is left alone.
    attach.wrist_throttle = None;
    let mut plain = rest.clone();
    roll_wrist(&attach, &mut plain, 1.0);
    assert_eq!(plain, rest);
}

#[test]
fn a_hand_keeps_its_place_and_turn_against_the_grip_bone_it_holds() {
    let mut limb = Resolved::new(&attach(), &skeleton()).unwrap().limbs[0];
    // The wrist is 10 cm behind the grip bone along the bone's own x axis, turned as it.
    limb.frame = Some(Transform::IDENTITY);
    limb.anchor = Some(Transform::from_rt(Quat::IDENTITY, Vec3::new(-0.1, 0.0, 0.0)));
    let turn = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let placed = Placed {
        position: Vec3::new(5.0, 5.0, 5.0),
        rotation: Some(Quat::IDENTITY),
        anchor: Some((Vec3::new(0.3, 0.2, 0.9), turn)),
    };
    let (position, rotation) = tip_goal(&limb, &placed);
    // The bone's x axis, turned a quarter about z, points along +y: the wrist is 10 cm behind the bone along it.
    assert!(position.distance(Vec3::new(0.3, 0.1, 0.9)) < 1e-6, "{position:?}");
    assert!(rotation.unwrap().angle_between(turn) < 1e-3);
    // Without the bike's anchor (an older file) the hand goes to the socket as it did.
    let socket_only = Placed { anchor: None, ..placed };
    assert!(tip_goal(&limb, &socket_only).0.distance(Vec3::new(5.0, 5.0, 5.0)) < 1e-6);
    // The throttle turns the grip bone's frame about the bone's own y axis, the grip's, and does not move it.
    let mut attach = attach();
    attach.targets.push(super::super::attach::Target {
        name: "right_grip".into(),
        position: [0.1, 0.1, 0.1],
        rotation: Some(turn.to_array()),
        steers: false,
        anchor: Some([0.3, 0.2, 0.9]),
        // A bone that is not the world's axes, to tell the bone's own y from the world's.
        anchor_rotation: Some(Quat::from_rotation_x(0.7).to_array()),
        grip_span: None,
    });
    attach.wrist_throttle = Some(super::super::attach::Rigid {
        translation: [0.0, 0.01, 0.0],
        rotation: Quat::from_rotation_z(0.5).to_array(),
    });
    let frame = plain_frame();
    let mut targets = rider_targets(&attach, &frame, None);
    let before = targets[2];
    roll_wrist(&attach, &mut targets, 1.0);
    let (at, bone) = (targets[2].anchor.unwrap(), before.anchor.unwrap());
    assert_eq!(at.0, bone.0, "the grip stays where it is, the hand goes round it: {at:?}");
    assert!((at.1.angle_between(bone.1) - 0.5).abs() < 1e-4, "{at:?}");
    // The turn is about the bone's own y axis, toward the rider (the other way from the bone's +y).
    let axis = (bone.1 * Vec3::Y).normalize();
    assert!((at.1 * Vec3::Y).distance(axis) < 1e-5, "the bone's y axis is the turn's axis");
    assert!((at.1 * Vec3::X).distance(Quat::from_axis_angle(axis, -0.5) * (bone.1 * Vec3::X)) < 1e-5);
}

/// The grip rubber's `[inboard, outboard]` span along its axis, metres from the grip bone, on real bikes (the importer's measure on the
/// bikes' chassis meshes: CRF 450 R SMT, RC 390, Panigale V4 S, KTM 690, Challenger, Africa Twin, Dragster 200, S 1000 RR) and the
/// extent of the posed glove along the grip from the same bone, measured on all eight with the probe of `probe_tests.rs`.
const GRIPS: [[f32; 2]; 8] =
    [[0.0, 0.116], [0.0, 0.121], [0.0, 0.125], [0.0, 0.113], [0.0, 0.129], [0.0, 0.137], [-0.010, 0.109], [0.0, 0.124]];
const GLOVE_EXTENT: [f32; 2] = [-0.046, 0.094];

fn grip(name: &str, span: Option<[f32; 2]>) -> super::super::attach::Target {
    super::super::attach::Target {
        name: name.into(),
        position: [0.1, 0.1, 0.1],
        rotation: Some(Quat::IDENTITY.to_array()),
        steers: false,
        anchor: Some([0.3, 0.2, 0.9]),
        // A bone whose y is the grip's axis, turned as the real ones are (14 degrees about z).
        anchor_rotation: Some(Quat::from_rotation_z(0.25).to_array()),
        grip_span: span,
    }
}

#[test]
fn the_glove_is_centred_on_the_grip_and_overhangs_neither_end_by_more_than_two_centimetres_on_real_grips() {
    for span in GRIPS {
        let slide = grip("right_grip", Some(span)).hand_slide();
        let inboard = (span[0] - (GLOVE_EXTENT[0] + slide)).max(0.0);
        let outboard = (GLOVE_EXTENT[1] + slide - span[1]).max(0.0);
        // Cutting into the throttle housing and the lever perch is the glove's inboard overhang: 4.6 cm without the slide.
        assert!(inboard <= 0.02 && outboard <= 0.02, "{span:?}: inboard {inboard}, outboard {outboard}");
        assert!(GLOVE_EXTENT[0].abs() - inboard > 0.025, "better than the 4.6 cm of the hand held by the bone alone");
    }
    // A file with no span assumes a grip of the usual length.
    let usual = grip("right_grip", None).hand_slide();
    assert!(usual > 0.03 && usual < 0.04, "{usual}");
}

#[test]
fn each_hand_slides_outboard_along_its_own_bones_axis() {
    let mut attach = attach();
    attach.targets.push(grip("left_grip", Some([0.0, 0.12])));
    attach.targets.push(grip("right_grip", Some([0.0, 0.12])));
    let targets = rider_targets(&attach, &plain_frame(), None);
    let (left, right) = (targets[targets.len() - 2], targets[targets.len() - 1]);
    let (bone_left, bone_right) = (left.anchor.unwrap(), right.anchor.unwrap());
    let slide = 0.06 - super::super::attach::GLOVE_CENTRE_M;
    // The bone's axes turned by the same 0.25 rad; the left bone's y points outboard (+y of the car), the right bone's inboard. Both
    // hands also come forward by the same distance along the bone's x.
    let turn = Quat::from_rotation_z(0.25);
    let (x, y) = (turn * Vec3::X * super::super::attach::HAND_FORWARD_M, turn * Vec3::Y * slide);
    assert!((bone_left.0 - Vec3::new(0.3, 0.2, 0.9) - y - x).length() < 1e-6, "{bone_left:?}");
    assert!((bone_right.0 - Vec3::new(0.3, 0.2, 0.9) + y - x).length() < 1e-6, "{bone_right:?}");
}
