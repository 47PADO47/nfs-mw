//! Where the rider sits on the bike, what its limbs reach for and how the bike drives the animation: `attach.json`,
//! measured from the bike's skeleton and the rider's reference pose. The game reads it without naming any of the RIDE 6
//! bones: targets and limbs are listed by role, and the numbers the game's rider runtime needs are in the file
//! (`docs/formats/ride6-rider.md`, "attach.json").

use anyhow::{Context, Result};
use blackbox_skeleton::{Pose, Skeleton, Transform};
use glam::Vec3;
use serde_json::{Value, json};

use super::contacts;
use super::raw::JsonSkeleton;

/// The bike socket that is the seat: the rider's root bone sits there.
pub const SEAT_SOCKET: &str = "SOCKET_RIDER";

/// The points the limbs reach for: `(role, bike socket, steers with the handlebar)`. The grips are on the handlebar; the
/// pegs are on the frame.
pub const TARGETS: [(&str, &str, bool); 4] = [
    ("left_grip", "SOCKET_L_HANDLE_POSE", true),
    ("right_grip", "SOCKET_R_HANDLE_POSE", true),
    ("left_foot", "SOCKET_LEFT_FOOT", false),
    ("right_foot", "SOCKET_RIGHT_FOOT", false),
];

/// The limbs the animation blueprint reaches with inverse kinematics: its control rigs `CtrlRig_HandsIK` and
/// `CtrlRig_FeetCorrection` solve them with a full-body IK; the runtime uses a two-bone solve on the same bones.
/// `(name, root, mid, tip, pole bone, target role)`.
pub const LIMBS: [(&str, &str, &str, &str, &str, &str); 4] = [
    ("left_arm", "upperarm_l", "lowerarm_l", "hand_l", "PV_arm_l", "left_grip"),
    ("right_arm", "upperarm_r", "lowerarm_r", "hand_r", "PV_arm_r", "right_grip"),
    ("left_leg", "thigh_l", "calf_l", "foot_l", "PV_leg_l", "left_foot"),
    ("right_leg", "thigh_r", "calf_r", "foot_r", "PV_leg_r", "right_foot"),
];

/// The bones whose rest positions are the two hubs: the wheels' visual bones.
pub const HUB_BONES: (&str, &str) = ("BONE_F_VIS_WHEEL", "BONE_B_VIS_WHEEL");

/// The clip whose first frame is the authored pose the tip offsets are measured on: the supermoto's centre pose at low
/// speed, the pose the lean sets blend away from.
pub const REFERENCE_CLIP: &str = "motard_CenterPose_LowSpeed";

/// The rider bone the inertia moves: the pelvis carries the legs, the spine and so the arms and head.
const INERTIA_BONE: &str = "pelvis";

/// `InertiaDisplacement_RidingMin` and `Max` of the animation blueprint, centimetres in the rider's frame of Unreal (left,
/// forward, up) as `[left, forward, up]`, here in metres: the rider's body moves 15 cm sideways, 0 to 6 cm back and -4
/// to 20 cm up. Data.
const INERTIA_RIDING: ([f32; 3], [f32; 3]) = ([-0.15, -0.06, -0.04], [0.15, 0.0, 0.20]);
/// `InertiaDisplacement_SmoothSpeed`: the rate at which the displacement follows its target, 1/s. Data.
const INERTIA_RATE: f32 = 4.0;
/// How far forward the rider moves under full braking, metres. A **choice**: the data's riding range only goes backwards
/// (accelerating); the braking range is not in the class defaults read, so it mirrors the riding one.
const INERTIA_BRAKING_M: f32 = 0.06;
/// The acceleration that moves the rider the full distance, m/s^2. A **choice**: one g, about what a tyre of friction one
/// can brake or accelerate at (the data's `5..40` range of the native input has no stated unit).
const INERTIA_FULL_ACCELERATION: f32 = 9.81;

/// A point of the bike's frame as a point of the rider's component space, for a rider whose root sits at `seat`.
#[must_use]
pub fn to_rider_space(seat: &Transform, point: Vec3) -> Vec3 {
    seat.inverse().transform_point(point)
}

/// Where the tip of a limb sits against its target in the authored pose, in the rider's component space: the vector from
/// the target to the tip. The runtime aims the tip at the target's new place plus this, so the pose's own placement of the
/// hand on the grip (and of the foot on the peg) is kept.
#[must_use]
pub fn tip_offset(tip_in_pose: Vec3, seat: &Transform, target: Vec3) -> Vec3 {
    tip_in_pose - to_rider_space(seat, target)
}

/// The attachment data as JSON. All transforms are in the car frame, metres, relative to the bike skeleton's origin (the
/// mesh origin) at the bike's rest pose; offsets and limb data are in the rider's component space. `reference` is the
/// rider's authored pose the tip offsets are measured on ([`REFERENCE_CLIP`]).
pub fn attach_json(rider: &Skeleton, reference: &Pose, bike: &JsonSkeleton) -> Result<Value> {
    let points = contacts::points(bike)?;
    let seat = points.seat;
    let rider_component = reference.component(rider)?;
    let mut limbs = Vec::new();
    for (name, root, mid, tip, pole, role) in LIMBS {
        for bone in [root, mid, tip, pole] {
            rider.find(bone).with_context(|| format!("the rider has no bone {bone}"))?;
        }
        let target = points.target(role)?.translation;
        let tip_index = rider.find(tip).with_context(|| format!("the rider has no bone {tip}"))?;
        let offset = tip_offset(rider_component[tip_index].translation, &seat, target);
        limbs.push(json!({
            "name": name, "root": root, "mid": mid, "tip": tip, "pole": pole, "target": role,
            "tip_offset": offset.to_array(),
        }));
    }
    rider.find(INERTIA_BONE).with_context(|| format!("the rider has no bone {INERTIA_BONE}"))?;
    // Unreal's rider frame is (left, forward, up); the file's inertia axes are the bike's (forward, left, up).
    let to_bike = |v: [f32; 3]| [v[1], v[0], v[2]];
    let mut json = json!({
        "frame": "car: x forward, y left, z up, metres, bike rest pose, origin of the bike skeleton",
        "limbs": limbs,
        "reference_clip": REFERENCE_CLIP,
        "inertia": {
            "bone": INERTIA_BONE,
            "min": to_bike(INERTIA_RIDING.0),
            "max": to_bike(INERTIA_RIDING.1),
            "braking_m": INERTIA_BRAKING_M,
            "full_acceleration": INERTIA_FULL_ACCELERATION,
            "rate": INERTIA_RATE,
        },
    });
    if let Some(map) = json.as_object_mut() {
        map.extend(points.placement());
    }
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rider::raw::{JsonBone, JsonSocket};
    use glam::Quat;

    fn bone(name: &str, parent: i32, t: [f32; 3]) -> JsonBone {
        JsonBone { name: name.to_owned(), parent, t, q: [0.0, 0.0, 0.0, 1.0], s: [1.0; 3] }
    }

    fn socket(name: &str, bone: &str, t: [f32; 3]) -> JsonSocket {
        JsonSocket { name: name.to_owned(), bone: bone.to_owned(), t, r: [0.0; 3], s: [1.0; 3] }
    }

    /// A bike with a chassis, two hubs and one socket for every target, in Unreal's centimetres.
    fn bike() -> JsonSkeleton {
        JsonSkeleton {
            bones: vec![
                bone("BONE_CHASSIS", -1, [0.0, 0.0, 0.0]),
                bone("BONE_F_VIS_WHEEL", 0, [73.0, 0.0, 22.0]),
                bone("BONE_B_VIS_WHEEL", 0, [-74.0, 0.0, 23.0]),
            ],
            sockets: vec![
                socket("SOCKET_RIDER", "BONE_CHASSIS", [-34.0, 0.0, 91.0]),
                socket("SOCKET_L_HANDLE_POSE", "BONE_CHASSIS", [12.0, -33.0, 114.0]),
                socket("SOCKET_R_HANDLE_POSE", "BONE_CHASSIS", [13.0, 33.0, 115.0]),
                socket("SOCKET_LEFT_FOOT", "BONE_CHASSIS", [-26.0, -21.0, 42.0]),
                socket("SOCKET_RIGHT_FOOT", "BONE_CHASSIS", [-24.0, 19.0, 43.0]),
            ],
        }
    }

    /// A rider with `root` and, for each limb, the four bones the limb names.
    fn rider() -> Skeleton {
        let mut bones = vec![blackbox_skeleton::Bone { name: "root".into(), parent: None, rest: Transform::IDENTITY }];
        bones.push(blackbox_skeleton::Bone { name: "pelvis".into(), parent: Some(0), rest: Transform::IDENTITY });
        for (_, root, mid, tip, pole, _) in LIMBS {
            for (name, parent) in [(root, 1), (mid, bones.len()), (tip, bones.len() + 1), (pole, 0)] {
                let rest = Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.3));
                bones.push(blackbox_skeleton::Bone { name: name.into(), parent: Some(parent), rest });
            }
        }
        Skeleton::new(bones).unwrap()
    }

    #[test]
    fn the_seat_turns_the_rider_to_face_forward_and_a_bike_point_maps_into_rider_space() {
        // The seat socket in the bike's frame: 90 degrees about z, at (0.1, 0, 0.9).
        let seat = Transform::from_rt(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), Vec3::new(0.1, 0.0, 0.9));
        // One metre ahead of the seat is, for a rider whose own axis y points forward on the bike... the rider's -y axis:
        // the rotation by +90 degrees takes the rider's -y to the bike's +x.
        let ahead = to_rider_space(&seat, Vec3::new(1.1, 0.0, 0.9));
        assert!(ahead.distance(Vec3::new(0.0, -1.0, 0.0)) < 1e-5, "{ahead:?}");
    }

    #[test]
    fn the_tip_offset_is_zero_when_the_pose_puts_the_tip_on_the_target() {
        let seat = Transform::from_rt(Quat::from_rotation_z(1.0), Vec3::new(0.2, 0.1, 0.8));
        let target = Vec3::new(0.5, -0.3, 1.1);
        let tip = to_rider_space(&seat, target);
        assert!(tip_offset(tip, &seat, target).length() < 1e-5);
        let moved = tip_offset(tip + Vec3::new(0.0, 0.0, 0.04), &seat, target);
        assert!(moved.distance(Vec3::new(0.0, 0.0, 0.04)) < 1e-5);
    }

    #[test]
    fn the_file_names_every_target_limb_hub_and_input_the_game_needs() {
        let rider = rider();
        let reference = rider.rest_pose();
        let json = attach_json(&rider, &reference, &bike()).unwrap();
        assert_eq!(json["targets"].as_array().unwrap().len(), 4);
        assert_eq!(json["limbs"].as_array().unwrap().len(), 4);
        let steers: Vec<bool> =
            json["targets"].as_array().unwrap().iter().map(|t| t["steers"].as_bool().unwrap()).collect();
        assert_eq!(steers, [true, true, false, false]);
        // The right grip is on the right: negative y in the car frame.
        assert!(json["targets"][1]["position"][1].as_f64().unwrap() < 0.0);
        // The front hub is ahead of the rear one, in metres and the car frame.
        let (front, rear) = (json["hubs"]["front"][0].as_f64().unwrap(), json["hubs"]["rear"][0].as_f64().unwrap());
        assert!((front - 0.73).abs() < 1e-5 && (rear + 0.74).abs() < 1e-5);
        assert_eq!(json["limbs"][0]["target"], "left_grip");
        assert!(json.get("inputs").is_none(), "the smoothing rates are the animation script's, not the file's");
        // The inertia ranges are the bike's axes: the forward range is the data's (-6 cm, 0).
        assert!((json["inertia"]["min"][0].as_f64().unwrap() + 0.06).abs() < 1e-6);
        assert!((json["inertia"]["max"][1].as_f64().unwrap() - 0.15).abs() < 1e-6);
        assert!(json["rider_root_in_bike"]["translation"].is_array());
    }

    #[test]
    fn a_rider_without_a_limb_bone_or_a_bike_without_a_socket_is_refused() {
        let rider = rider();
        let mut bike = bike();
        bike.sockets.retain(|s| s.name != "SOCKET_LEFT_FOOT");
        assert!(attach_json(&rider, &rider.rest_pose(), &bike).is_err());
        let few = Skeleton::new(vec![blackbox_skeleton::Bone {
            name: "root".into(),
            parent: None,
            rest: Transform::IDENTITY,
        }])
        .unwrap();
        assert!(attach_json(&few, &few.rest_pose(), &self::bike()).is_err());
    }
}
