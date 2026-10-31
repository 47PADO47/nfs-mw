use blackbox_skeleton::fall::{Launch, Rest};
use blackbox_skeleton::{Bone, Channel, Clip, FallSet, Graph, Node, Pack, TimeMode, Track};
use blackbox_vehicle::{FIXED_STEP, FlatGround};

use super::*;

/// A rider reduced to a spine and a leg: a root, a pelvis a metre up, a spine and a head above it, a thigh and a calf below.
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
        bone("head", Some(2), Vec3::new(0.0, 0.0, 0.4)),
        bone("thigh_l", Some(1), Vec3::new(0.0, 0.1, -0.05)),
        bone("calf_l", Some(4), Vec3::new(0.0, 0.0, -0.45)),
    ])
    .unwrap()
}

const FILE: &str = r#"{
    "bodies": [
        {"bone": "pelvis", "mass": 7.0, "radius": 0.11, "half_length": 0.075, "centre": [0, 0, 0], "axis": [0, 1, 0]},
        {"bone": "spine_02", "mass": 20.0, "radius": 0.19, "half_length": 0.1, "centre": [0, 0, 0.1], "axis": [0, 1, 0]},
        {"bone": "head", "mass": 4.3, "radius": 0.147, "half_length": 0.0125, "centre": [0, 0, 0.1], "axis": [0, 0, 1]},
        {"bone": "thigh_l", "mass": 7.0, "radius": 0.07, "half_length": 0.125, "centre": [0, 0, -0.2], "axis": [0, 0, 1]},
        {"bone": "calf_l", "mass": 4.0, "radius": 0.065, "half_length": 0.135, "centre": [0, 0, -0.2], "axis": [0, 0, 1]}
    ],
    "joints": [
        {"child": "spine_02", "parent": "pelvis", "frame_child": [0, 0, 0, 1], "frame_parent": [0, 0, 0, 1],
         "swing_z_deg": 180, "swing_y_deg": 180, "twist_deg": 180},
        {"child": "head", "parent": "spine_02", "frame_child": [0, 0, 0, 1], "frame_parent": [0, 0, 0, 1],
         "swing_z_deg": 180, "swing_y_deg": 180, "twist_deg": 180},
        {"child": "thigh_l", "parent": "pelvis", "frame_child": [0, 0, 0, 1], "frame_parent": [0, 0, 0, 1],
         "swing_z_deg": 180, "swing_y_deg": 180, "twist_deg": 180},
        {"child": "calf_l", "parent": "thigh_l", "frame_child": [0, 0, 0, 1], "frame_parent": [0, 0, 0, 1],
         "swing_z_deg": 180, "swing_y_deg": 180, "twist_deg": 180}
    ],
    "contact_friction": 0.66,
    "joint_damping": 4.0,
    "falls": {"velocity_multiplier": 1.0,
              "high_side": {"additional": [0, 0, 1.6667], "scale": [0.05, 0, -0.28], "world": [0, 0, 0], "angular": 0.5},
              "low_side": {"additional": [0, 0, 0], "scale": [0.05, 0, 0.32], "world": [0, 0, 0], "angular": 0.5}}
}"#;

fn file() -> RagdollFile {
    RagdollFile::parse(FILE.as_bytes()).unwrap()
}

fn release(kind: FallKind) -> RiderRelease {
    RiderRelease {
        kind,
        position: Vec3::new(0.0, 1.2, 0.0),
        // 15 m/s forward in the physics space (+z), and a roll about the bike's length.
        velocity: Vec3::new(0.0, 0.0, 15.0),
        angular_velocity: Vec3::new(0.0, 0.0, 2.0),
        orientation: Quat::IDENTITY,
    }
}

/// The rider seated 0.2 m above the ground in the render space (x forward, y left, z up).
fn seated() -> Mat4 {
    Mat4::from_translation(Vec3::new(0.0, 0.0, 0.2))
}

fn riding(skeleton: &Skeleton) -> Vec<Transform> {
    let mut out = Vec::new();
    skeleton.component_transforms(&skeleton.rest_pose().local, &mut out).unwrap();
    out
}

fn started(kind: FallKind) -> (Fall, Skeleton) {
    let skeleton = skeleton();
    let mut fall = Fall::new(Some(file()));
    fall.remember(&riding(&skeleton));
    assert!(fall.start(&skeleton, (seated(), Quat::IDENTITY), &release(kind)));
    (fall, skeleton)
}

fn run(fall: &mut Fall, seconds: f32) {
    let ground = FlatGround::new(0.0);
    for _ in 0..(seconds / FIXED_STEP) as usize {
        fall.step(FIXED_STEP, &ground);
    }
}

#[test]
fn a_rider_without_a_ragdoll_keeps_to_his_bike() {
    let skeleton = skeleton();
    let mut fall = Fall::default();
    fall.remember(&riding(&skeleton));
    assert!(!fall.can_fall());
    assert!(!fall.start(&skeleton, (seated(), Quat::IDENTITY), &release(FallKind::LowSide)));
    assert!(!fall.active() && fall.pose(&skeleton, &riding(&skeleton), seated()).is_none());
}

#[test]
fn the_rider_leaves_with_the_velocity_of_the_bike_changed_by_the_kind_of_fall() {
    let bike = Quat::IDENTITY;
    let falls = file().falls;
    // A fall of no particular kind: the bike's velocity, no spin.
    let (v, spin) = launch(&release(FallKind::Generic), &falls, bike);
    assert!((v - Vec3::new(0.0, 0.0, 15.0)).length() < 1e-5 && spin == Vec3::ZERO);
    // A low side: 5 percent more along, and the vertical part of the velocity (none here) scaled; the spin is half.
    let (v, spin) = launch(&release(FallKind::LowSide), &falls, bike);
    assert!((v - Vec3::new(0.0, 0.0, 15.75)).length() < 1e-4, "{v:?}");
    assert_eq!(spin, Vec3::new(0.0, 0.0, 1.0));
    // A high side: the same, and thrown 6 km/h up (1.67 m/s) in the rider's frame.
    let (v, _) = launch(&release(FallKind::HighSide), &falls, bike);
    assert!((v - Vec3::new(0.0, 1.6667, 15.75)).length() < 1e-3, "{v:?}");
    // The configuration is in the bike's frame: a bike turned a quarter to the left throws the same way in its own axes.
    let left = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let release = RiderRelease { velocity: to_physics(left * Vec3::X * 15.0), ..release(FallKind::HighSide) };
    let (v, _) = launch(&release, &falls, left);
    assert!((v - Vec3::new(-15.75, 1.6667, 0.0)).length() < 1e-3, "{v:?}");
}

#[test]
fn the_ragdoll_starts_where_the_rider_sits_with_his_velocity_and_every_body_moves_with_it() {
    let (fall, skeleton) = started(FallKind::LowSide);
    let State::Down(tumble) = &fall.state else { panic!("not down") };
    assert_eq!(tumble.ragdoll.len(), 5);
    // Whole-body velocity: 15.75 m/s forward (+z in the physics space).
    let v = tumble.ragdoll.centre_velocity();
    assert!((v.z - 15.75).abs() < 0.2 && v.x.abs() < 0.2, "{v:?}");
    // The pelvis body is at the pelvis bone (1 m up the seat, 0.2 above the ground), in the physics space.
    let pelvis = to_render(tumble.ragdoll.position(0));
    assert!(pelvis.distance(Vec3::new(0.0, 0.0, 1.2)) < 1e-4, "{pelvis:?}");
    // Before anything moves the pose is the riding pose, shifted to the origin the instance is placed at.
    let mut fall = fall;
    let pose = fall.pose(&skeleton, &riding(&skeleton), seated()).expect("down");
    let origin = fall.instance().transform_point3(Vec3::ZERO);
    let rest = riding(&skeleton);
    for bone in 0..skeleton.len() {
        let moved = pose[bone].translation + origin - (rest[bone].translation + Vec3::new(0.0, 0.0, 0.2));
        assert!(moved.length() < 1e-3, "bone {bone}: {moved:?}");
    }
}

#[test]
fn the_rider_falls_and_slides_over_the_ground_and_comes_to_rest() {
    let (mut fall, skeleton) = started(FallKind::LowSide);
    run(&mut fall, 0.5);
    let State::Down(tumble) = &fall.state else { panic!("not down") };
    let ahead = to_render(tumble.ragdoll.centre()).x;
    assert!(ahead > 5.0, "he travelled forward: {ahead}");
    run(&mut fall, 10.0);
    let State::Down(tumble) = &fall.state else { panic!("not down") };
    assert!(tumble.ragdoll.at_rest() && tumble.ragdoll.is_finite());
    let end = to_render(tumble.ragdoll.centre());
    assert!(end.x > ahead + 1.0, "he slid on: {end:?}");
    assert!(end.z < 0.5, "he lies on the ground: {end:?}");
    let pose = fall.pose(&skeleton, &riding(&skeleton), seated()).expect("down");
    assert!(pose.iter().all(|t| t.translation.is_finite() && t.rotation.is_finite()));
}

#[test]
fn what_hangs_on_a_body_follows_it_like_it_did_in_the_riding_pose() {
    let (mut fall, skeleton) = started(FallKind::LowSide);
    run(&mut fall, 1.0);
    let pose = fall.pose(&skeleton, &riding(&skeleton), seated()).expect("down");
    // The root has no body: it is the riding pose's, and the calf (a body) is where its body is. The calf's length is kept: the
    // thigh and the calf stay joined.
    let thigh_to_calf = pose[5].translation - pose[4].translation;
    assert!((thigh_to_calf.length() - 0.45).abs() < 0.05, "{}", thigh_to_calf.length());
    // The head and the spine stay joined too.
    let spine_to_head = pose[3].translation - pose[2].translation;
    assert!((spine_to_head.length() - 0.4).abs() < 0.06, "{}", spine_to_head.length());
}

#[test]
fn standing_the_bike_up_puts_the_rider_back_on_it_with_a_short_blend() {
    let (mut fall, skeleton) = started(FallKind::HighSide);
    run(&mut fall, 2.0);
    assert!(fall.is_down() && fall.active());
    let sitting = riding(&skeleton);
    // Down: the pose is the ragdoll's, far from the seat.
    let lying = fall.pose(&skeleton, &sitting, seated()).expect("down");
    let seat = Vec3::new(0.0, 0.0, 0.2);
    let away =
        (lying[1].translation + fall.instance().transform_point3(Vec3::ZERO)).distance(seat + sitting[1].translation);
    assert!(away > 3.0, "{away}");
    fall.stop(&skeleton);
    assert!(fall.active() && !fall.is_down());
    // At the start of the blend he is where he lay; half way he is between; at the end he is on the bike and the fall is over.
    let start = fall.pose(&skeleton, &sitting, seated()).expect("returning");
    let world = |pose: &[Transform], fall: &Fall| pose[1].translation + fall.instance().transform_point3(Vec3::ZERO);
    let first = world(&start, &fall);
    assert!(first.distance(seat + sitting[1].translation) > 3.0);
    let ground = FlatGround::new(0.0);
    fall.step(RETURN_TIME / 2.0, &ground);
    let middle = fall.pose(&skeleton, &sitting, seated()).expect("returning");
    let half = world(&middle, &fall);
    assert!(half.distance(seat + sitting[1].translation) < first.distance(seat + sitting[1].translation));
    fall.step(RETURN_TIME, &ground);
    assert!(!fall.active(), "back on the bike");
    assert!(fall.pose(&skeleton, &sitting, seated()).is_none());
}

/// A clip that holds `rotation` on `bone` (and `lift` on the pelvis) for `length` seconds.
fn pose_clip(name: &str, length: f32, bone: usize, rotation: Quat, lift: f32) -> Clip {
    let mut tracks =
        vec![Track { bone, rotation: Channel::Constant(rotation), translation: Channel::None, scale: Channel::None }];
    if lift != 0.0 {
        tracks.push(Track {
            bone: 1,
            rotation: Channel::None,
            translation: Channel::Constant(Vec3::new(0.0, 0.0, 1.0 + lift)),
            scale: Channel::None,
        });
    }
    Clip { name: name.into(), length, tracks, additive: false, root_motion: false, notifies: Vec::new() }
}

const HEAD_TURN: f32 = 0.8;

/// A pack with a launch clip, a tumble clip (the head turned `HEAD_TURN` about the rider's Y axis) and a get-up clip that stands the
/// pelvis 0.5 m higher, and the set that plays them: the get-up begins as soon as the rider moves slower than `low_speed`.
fn animated(low_speed: f32) -> (Pack, FallSet) {
    let skeleton = skeleton();
    let clips = vec![
        pose_clip("launch", 1.0, 3, Quat::from_rotation_y(-0.3), 0.0),
        pose_clip("tumble", 1.0, 3, Quat::from_rotation_y(HEAD_TURN), 0.0),
        pose_clip("get_up", 1.0, 3, Quat::IDENTITY, 0.5),
    ];
    let clip = |clip: usize, time: TimeMode| Graph { param_names: Vec::new(), root: Node::Clip { clip, time } };
    let set = FallSet {
        launches: vec![Launch { kind: "high_side".into(), graph: clip(0, TimeMode::Once { rate: 1.0 }) }],
        launch_blend: 0.6,
        tumble: clip(1, TimeMode::Loop { rate: 1.0 }),
        rests: vec![Rest {
            face_above: -2.0,
            face_below: 2.0,
            graph: clip(2, TimeMode::Once { rate: 1.0 }),
            blend: 0.5,
        }],
        low_speed,
        flying_above: 1.0,
        flying_ramp: (1.0, 2.5),
        fade_rate: 3.0,
    };
    (Pack { skeleton, sockets: Vec::new(), clips, meshes: Vec::new() }, set)
}

fn started_animated(kind: FallKind, low_speed: f32) -> (Fall, Pack) {
    let (pack, set) = animated(low_speed);
    let mut fall = Fall { set: Some(set), ..Fall::new(Some(file())) };
    fall.remember(&riding(&pack.skeleton));
    assert!(fall.start(&pack.skeleton, (seated(), Quat::IDENTITY), &release(kind)));
    (fall, pack)
}

fn run_animated(fall: &mut Fall, pack: &Pack, seconds: f32) {
    let ground = FlatGround::new(0.0);
    for _ in 0..(seconds / FIXED_STEP) as usize {
        fall.animate(pack, FIXED_STEP, &ground);
        fall.step(FIXED_STEP, &ground);
    }
}

/// Run until the get-up begins (at most 20 s).
fn run_until_getting_up(fall: &mut Fall, pack: &Pack) {
    let ground = FlatGround::new(0.0);
    for _ in 0..(20.0 / FIXED_STEP) as usize {
        fall.animate(pack, FIXED_STEP, &ground);
        fall.step(FIXED_STEP, &ground);
        if matches!(&fall.state, State::Down(tumble) if tumble.frozen()) {
            return;
        }
    }
    panic!("he never lay still");
}

/// The angle of the head against the spine in a pose (rad).
fn head_turn(pose: &[Transform]) -> f32 {
    (pose[2].rotation.inverse() * pose[3].rotation).angle_between(Quat::IDENTITY)
}

#[test]
fn a_fall_kind_is_named_for_the_animation() {
    assert_eq!(fall_anim::kind_name(FallKind::HighSide), "high_side");
    assert_eq!(fall_anim::kind_name(FallKind::LowSide), "low_side");
    assert_eq!(fall_anim::kind_name(FallKind::Generic), "generic");
}

#[test]
fn the_bodies_are_driven_to_the_pose_of_the_fall_animation() {
    let (mut fall, pack) = started_animated(FallKind::HighSide, 0.0);
    // At the start the target is the riding pose: the head sits as it did on the bike.
    run_animated(&mut fall, &pack, 0.1);
    let sitting = fall.pose(&pack.skeleton, &riding(&pack.skeleton), seated()).expect("down");
    assert!(head_turn(&sitting) < 0.3, "{}", head_turn(&sitting));
    // Two seconds on, the target is the tumble clip's (the fade is 95 percent after a second) and the head has turned to it against
    // the spine, though the whole body is tumbling over the road.
    run_animated(&mut fall, &pack, 2.0);
    let driven = fall.pose(&pack.skeleton, &riding(&pack.skeleton), seated()).expect("down");
    assert!((head_turn(&driven) - HEAD_TURN).abs() < 0.15, "{}", head_turn(&driven));
}

#[test]
fn a_rider_without_the_animation_falls_limp_as_before() {
    let pack = animated(0.0).0;
    let mut limp = Fall::new(Some(file()));
    limp.remember(&riding(&pack.skeleton));
    assert!(limp.start(&pack.skeleton, (seated(), Quat::IDENTITY), &release(FallKind::LowSide)));
    let State::Down(tumble) = &limp.state else { panic!("not down") };
    assert!(tumble.anim.is_none());
    run_animated(&mut limp, &pack, 1.0);
    let pose = limp.pose(&pack.skeleton, &riding(&pack.skeleton), seated()).expect("down");
    assert!(pose.iter().all(|t| t.translation.is_finite()));
}

#[test]
fn once_he_lies_still_the_get_up_plays_where_he_lies_and_the_body_is_no_longer_simulated() {
    let (mut fall, pack) = started_animated(FallKind::LowSide, 0.5);
    run_until_getting_up(&mut fall, &pack);
    let State::Down(tumble) = &fall.state else { panic!("not down") };
    assert!(tumble.frozen(), "he is in the get-up");
    let lying = to_render(tumble.ragdoll.centre());
    run_animated(&mut fall, &pack, 0.2);
    let State::Down(tumble) = &fall.state else { panic!("not down") };
    assert_eq!(to_render(tumble.ragdoll.centre()), lying, "the body no longer moves");
    // The clip stands the pelvis 0.5 m higher: once it has played the pelvis is that much over where it lay.
    let before = fall.pose(&pack.skeleton, &riding(&pack.skeleton), seated()).expect("down");
    run_animated(&mut fall, &pack, 1.5);
    let after = fall.pose(&pack.skeleton, &riding(&pack.skeleton), seated()).expect("down");
    assert!(
        after[1].translation.z > before[1].translation.z + 0.2,
        "{} {}",
        after[1].translation.z,
        before[1].translation.z
    );
    assert!(after.iter().all(|t| t.translation.is_finite() && t.rotation.is_finite()));
}

#[test]
fn the_bike_standing_up_waits_for_the_get_up_to_play_and_then_the_rider_gets_back_on() {
    let (mut fall, pack) = started_animated(FallKind::LowSide, 0.5);
    run_until_getting_up(&mut fall, &pack);
    // The bike has been stood up: he stays down while the get-up plays.
    fall.stop(&pack.skeleton);
    assert!(fall.is_down(), "still getting up");
    let ground = FlatGround::new(0.0);
    let mut steps = 0;
    while fall.is_down() && steps < 600 {
        fall.animate(&pack, FIXED_STEP, &ground);
        fall.step(FIXED_STEP, &ground);
        fall.stop(&pack.skeleton);
        steps += 1;
    }
    assert!(!fall.is_down() && fall.active(), "returning after {steps} steps");
    let waited = steps as f32 * FIXED_STEP;
    assert!((0.2..1.6).contains(&waited), "the get-up is a second long: {waited}");
}

#[test]
fn a_rider_who_has_not_begun_to_get_up_when_the_bike_is_stood_up_is_put_back_after_a_grace() {
    // He never gets slow enough: the bike stands up at once.
    let (mut fall, pack) = started_animated(FallKind::HighSide, 0.0);
    let ground = FlatGround::new(0.0);
    fall.stop(&pack.skeleton);
    assert!(fall.is_down(), "he is let lie for a while");
    let mut steps = 0;
    while fall.is_down() && steps < 600 {
        fall.animate(&pack, FIXED_STEP, &ground);
        fall.step(FIXED_STEP, &ground);
        fall.stop(&pack.skeleton);
        steps += 1;
    }
    let waited = steps as f32 * FIXED_STEP;
    assert!((fall_anim::RETURN_GRACE - 0.1..fall_anim::RETURN_GRACE + 0.1).contains(&waited), "{waited}");
}
