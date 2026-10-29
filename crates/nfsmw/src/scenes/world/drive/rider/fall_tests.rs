use blackbox_skeleton::Bone;
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
