//! The rider on bikes of very different shapes: a supermoto, a sport bike with clip-ons and a cruiser. A synthetic rider of human
//! proportions (a plain skeleton, no meshes) is posed on contact points of each, and the places of his pelvis, hands and feet
//! are measured against the seat, the grips and the pegs.

use blackbox_skeleton::{Bone, Clip, Graph, Node, Skeleton, TimeMode, Transform};
use glam::{Quat, Vec3};
use serde_json::json;

use super::attach::Attach;
use super::pose::{self, Body, Frame, Goals, Resolved};
use super::seat::Seat;

/// Where a bike has its seat socket, grips, pegs and seat top (the bike skeleton's frame: x forward, y left, z up).
struct Bike {
    seat: Vec3,
    /// The left grip and foot; the right ones mirror them.
    grip: Vec3,
    peg: Vec3,
    seat_top: f32,
}

/// The supermoto the animation is authored for: its sockets, as measured on the KTM 450 SMR (the left ones; the right ones are
/// within 2 cm of their mirror) and its seat's top 5.9 cm under the socket.
const SUPERMOTO: Bike = Bike {
    seat: Vec3::new(-0.3439, 0.0, 0.9126),
    grip: Vec3::new(0.1176, 0.3336, 1.1376),
    peg: Vec3::new(-0.2573, 0.2088, 0.4241),
    seat_top: 0.854,
};
/// A sport bike with clip-ons: the grips barely above the seat and far ahead of it, the pegs high and back, the socket 12.6 cm
/// above the seat's top (the numbers of the Yamaha R7).
const SPORT: Bike = Bike {
    seat: Vec3::new(-0.2712, 0.0, 0.9519),
    grip: Vec3::new(0.2605, 0.2879, 0.9325),
    peg: Vec3::new(-0.3367, 0.2182, 0.4968),
    seat_top: 0.826,
};
/// A cruiser: a low seat, high bars close to it, the pegs forward and low.
const CRUISER: Bike = Bike {
    seat: Vec3::new(-0.10, 0.0, 0.78),
    grip: Vec3::new(0.30, 0.45, 1.12),
    peg: Vec3::new(0.30, 0.25, 0.30),
    seat_top: 0.70,
};

/// A rider of human proportions in rider space (he faces +y, his left is +x, up is +z), every bone's rotation the identity so
/// that a child's rest translation is its offset in that space: a bone name and its parent and offset, then its length is that.
fn rider() -> Skeleton {
    let b = |name: &str, parent: Option<usize>, x: f32, y: f32, z: f32| Bone {
        name: name.into(),
        parent,
        rest: Transform::from_rt(Quat::IDENTITY, Vec3::new(x, y, z)),
    };
    Skeleton::new(vec![
        b("root", None, 0.0, 0.0, 0.0),
        b("pelvis", Some(0), 0.0, 0.0, 0.05),
        b("spine_01", Some(1), 0.0, 0.0, 0.12),
        b("spine_02", Some(2), 0.0, 0.0, 0.11),
        b("spine_03", Some(3), 0.0, 0.0, 0.13),
        b("clavicle_l", Some(4), 0.04, 0.0, 0.15),
        b("upperarm_l", Some(5), 0.14, 0.0, 0.0),
        b("lowerarm_l", Some(6), 0.0, 0.0, -0.27),
        b("hand_l", Some(7), 0.0, 0.0, -0.26),
        b("clavicle_r", Some(4), -0.04, 0.0, 0.15),
        b("upperarm_r", Some(9), -0.14, 0.0, 0.0),
        b("lowerarm_r", Some(10), 0.0, 0.0, -0.27),
        b("hand_r", Some(11), 0.0, 0.0, -0.26),
        b("neck_01", Some(4), 0.0, 0.0, 0.2),
        b("head", Some(13), 0.0, 0.0, 0.1),
        b("thigh_l", Some(1), 0.09, 0.0, -0.02),
        b("calf_l", Some(15), 0.0, 0.0, -0.38),
        b("foot_l", Some(16), 0.0, 0.0, -0.40),
        b("thigh_r", Some(1), -0.09, 0.0, -0.02),
        b("calf_r", Some(18), 0.0, 0.0, -0.38),
        b("foot_r", Some(19), 0.0, 0.0, -0.40),
        b("PV_arm_l", Some(0), 0.7, -0.3, 0.9),
        b("PV_arm_r", Some(0), -0.7, -0.3, 0.9),
        b("PV_leg_l", Some(0), 0.2, 0.9, 0.5),
        b("PV_leg_r", Some(0), -0.2, 0.9, 0.5),
    ])
    .unwrap()
}

/// The points of his body that rest on the seat, in his pelvis's frame: the underside of the buttocks, 11 to 12 cm under it.
fn seat_points() -> Vec<Vec3> {
    [-0.09, 0.09]
        .iter()
        .flat_map(|&x| [(-0.10, -0.12), (0.0, -0.12), (0.06, -0.10)].map(|(y, z)| Vec3::new(x, y, z)))
        .collect()
}

fn attach(bike: &Bike) -> Attach {
    let identity = [0.0, 0.0, 0.0, 1.0];
    let at = |p: Vec3| p.to_array();
    let mirror = |p: Vec3| Vec3::new(p.x, -p.y, p.z);
    let limb = |name: &str, side: &str, joints: [&str; 3], pole: &str, target: &str| {
        let [root, mid, tip] = joints.map(|j| format!("{j}_{side}"));
        json!({
            "name": name, "root": root, "mid": mid, "tip": tip, "pole": pole, "target": target,
            "tip_offset": [0.0, 0.0, 0.0],
            "tip_frame": {"translation": [0.0, 0.0, 0.0], "rotation": identity},
        })
    };
    let seat_x = bike.seat.x;
    Attach::parse(
        json!({
            "rider_root_in_bike": {"translation": at(bike.seat), "rotation": [0.0, 0.0, std::f32::consts::FRAC_1_SQRT_2, std::f32::consts::FRAC_1_SQRT_2]},
            "hubs": {"front": [0.75, 0.0, 0.3], "rear": [-0.75, 0.0, 0.3]},
            "targets": [
                {"name": "left_grip", "position": at(bike.grip), "rotation": identity, "steers": true},
                {"name": "right_grip", "position": at(mirror(bike.grip)), "rotation": identity, "steers": true},
                {"name": "left_foot", "position": at(bike.peg), "rotation": identity, "steers": false},
                {"name": "right_foot", "position": at(mirror(bike.peg)), "rotation": identity, "steers": false},
            ],
            "limbs": [
                limb("left_arm", "l", ["upperarm", "lowerarm", "hand"], "PV_arm_l", "left_grip"),
                limb("right_arm", "r", ["upperarm", "lowerarm", "hand"], "PV_arm_r", "right_grip"),
                limb("left_leg", "l", ["thigh", "calf", "foot"], "PV_leg_l", "left_foot"),
                limb("right_leg", "r", ["thigh", "calf", "foot"], "PV_leg_r", "right_foot"),
            ],
            "inertia": {"bone": "pelvis", "min": [-0.06, -0.15, -0.04], "max": [0.0, 0.15, 0.2], "rate": 4.0},
            "torso": {"lean": ["spine_01", "spine_02", "spine_03"], "head": ["neck_01", "head"], "head_share": 0.8},
            "seat": {"bones": ["pelvis"], "sink": 0.02},
            "seat_surface": [[seat_x - 0.3, bike.seat_top], [seat_x + 0.1, bike.seat_top]],
        })
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}

/// The rider's pose on `bike`: the component transforms, the placed targets and the frame.
fn ride(bike: &Bike, sit: bool) -> (Vec<Transform>, Vec<pose::Placed>, Frame, Resolved) {
    let attach = attach(bike);
    let skeleton = rider();
    let mut resolved = Resolved::new(&attach, &skeleton).unwrap();
    if sit {
        resolved.seat =
            Some(Seat::from_points(1, seat_points(), attach.seat_surface.clone(), attach.seat.as_ref().unwrap().sink));
    }
    let frame = Frame::new(&attach.rider_root_in_bike.transform(), Vec3::ZERO);
    let targets = pose::rider_targets(&attach, &frame, None);
    let clip = Clip {
        name: "rest".into(),
        length: 1.0,
        tracks: Vec::new(),
        additive: false,
        root_motion: false,
        notifies: Vec::new(),
    };
    let graph = Graph { param_names: Vec::new(), root: Node::Clip { clip: 0, time: TimeMode::Hold } };
    let goals = Goals { targets: &targets, body: Body::REST, frame: &frame };
    let component = pose::animate(&skeleton, &[clip], &graph, (&[], 0.0), &resolved, &goals).unwrap();
    (component, targets, frame, resolved)
}

/// How far each limb's tip is from the place it was asked to be at, metres, in the order of the file's limbs.
fn misses(component: &[Transform], targets: &[pose::Placed], resolved: &Resolved) -> Vec<f32> {
    resolved
        .limbs
        .iter()
        .map(|limb| component[limb.chain.tip].translation.distance(pose::tip_goal(limb, &targets[limb.target]).0))
        .collect()
}

/// How far the lowest seat point is above the seat's top in the pose (negative: sunk into it).
fn clearance(component: &[Transform], frame: &Frame, bike: &Bike) -> f32 {
    let attach = attach(bike);
    let seat = Seat::from_points(1, seat_points(), attach.seat_surface, 0.02);
    seat.clearance(component, &frame.to_bike).unwrap()
}

#[test]
fn on_every_bike_the_buttocks_rest_in_the_seat_whatever_height_its_socket_is_at() {
    for (name, bike) in [("supermoto", &SUPERMOTO), ("sport", &SPORT), ("cruiser", &CRUISER)] {
        let (component, _, frame, _) = ride(bike, true);
        let sunk = clearance(&component, &frame, bike);
        assert!(
            (sunk + 0.02).abs() < 0.005,
            "{name}: the lowest point is {:.1} cm from sinking 2 cm",
            (sunk + 0.02) * 100.0
        );
        // The pelvis is where a hip is over a seat: the contact points are 12 cm under it.
        let pelvis_in_bike = frame.to_bike.transform_point3(component[1].translation);
        assert!(
            (pelvis_in_bike.z - (bike.seat_top - 0.02 + 0.12)).abs() < 0.005,
            "{name}: the pelvis is at {:.3} m",
            pelvis_in_bike.z
        );
    }
}

#[test]
fn without_a_seat_surface_the_rider_sits_at_the_socket_as_before() {
    let (component, _, frame, _) = ride(&SPORT, false);
    // The root is on the socket and the pelvis 5 cm over it, though the seat is 12.6 cm under the socket.
    let pelvis = frame.to_bike.transform_point3(component[1].translation);
    assert!((pelvis.z - (SPORT.seat.z + 0.05)).abs() < 1e-4, "{pelvis:?}");
    assert!(clearance(&component, &frame, &SPORT) > 0.05, "he hovers: that is what the seat surface fixes");
}

#[test]
fn the_hands_are_on_the_grips_and_the_feet_on_the_pegs_within_a_centimetre_on_every_bike() {
    for (name, bike) in [("supermoto", &SUPERMOTO), ("sport", &SPORT), ("cruiser", &CRUISER)] {
        let (component, targets, _, resolved) = ride(bike, true);
        for (limb, miss) in resolved.limbs.iter().zip(misses(&component, &targets, &resolved)) {
            assert!(miss < 0.01, "{name}: limb to target {} misses by {:.1} cm", limb.target, miss * 100.0);
        }
    }
}

#[test]
fn a_hand_takes_the_turn_of_its_grip_and_the_arm_keeps_its_bones() {
    let (component, targets, frame, resolved) = ride(&SPORT, true);
    let hand = resolved.limbs[0].chain.tip;
    // The grip's frame is the bike's own (identity), so the hand's turn in the bike is the identity too.
    let in_bike = Quat::from_mat4(&frame.to_bike) * component[hand].rotation;
    assert!(in_bike.angle_between(Quat::IDENTITY) < 1e-4, "{in_bike:?}");
    assert!(targets[0].rotation.is_some());
    for limb in &resolved.limbs {
        let at = |bone: usize| component[bone].translation;
        let (upper, lower) =
            (at(limb.chain.mid).distance(at(limb.chain.root)), at(limb.chain.tip).distance(at(limb.chain.mid)));
        let (rest_upper, rest_lower) = match limb.on_torso {
            true => (0.27, 0.26),
            false => (0.38, 0.40),
        };
        assert!((upper - rest_upper).abs() < 1e-4 && (lower - rest_lower).abs() < 1e-4, "{upper} {lower}");
    }
}

#[test]
fn the_sport_bike_leans_its_rider_further_than_the_supermoto_and_keeps_his_elbows_bent() {
    // The angle of the spine from upright: the shoulders over the pelvis, in the bike's frame, 0 for straight up.
    let spine = |component: &[Transform], frame: &Frame| {
        let up = frame.to_bike.transform_point3(component[4].translation)
            - frame.to_bike.transform_point3(component[1].translation);
        up.x.atan2(up.z)
    };
    let (moto, _, frame, _) = ride(&SUPERMOTO, true);
    let (sport, targets, frame_sport, resolved) = ride(&SPORT, true);
    let (lean_moto, lean_sport) = (spine(&moto, &frame), spine(&sport, &frame_sport));
    assert!(
        lean_sport > lean_moto + 0.2,
        "the sport bike's rider is down on the tank: {lean_sport} against {lean_moto} rad"
    );
    assert!(lean_sport < super::reach::MAX_LEAN, "{lean_sport}");
    // Every arm is bent at the elbow (shoulder to hand shorter than the arm by the reach fraction), never stretched straight.
    for limb in resolved.limbs.iter().filter(|l| l.on_torso) {
        let shoulder = sport[limb.chain.root].translation;
        let hand = sport[limb.chain.tip].translation;
        let wanted = pose::tip_goal(limb, &targets[limb.target]).0;
        assert!(hand.distance(wanted) < 0.01);
        assert!(shoulder.distance(hand) <= 0.53 * super::reach::REACH_FRACTION + 0.01, "{}", shoulder.distance(hand));
    }
    // The head stays up: the helmet is well above the pelvis, though he is down on the tank.
    let head = frame_sport.to_bike.transform_point3(sport[14].translation);
    let pelvis = frame_sport.to_bike.transform_point3(sport[1].translation);
    assert!(head.z - pelvis.z > 0.45, "{head:?} {pelvis:?}");
}
