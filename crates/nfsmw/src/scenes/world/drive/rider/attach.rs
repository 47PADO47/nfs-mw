//! `attach.json`: where a rider sits on a two-wheeler, what its limbs reach for and the numbers that drive its animation.
//! The file is written by a mod's converter and read here without knowing what game the rider comes from: the targets and
//! limbs are listed by role and by the rider skeleton's own bone names (`docs/formats/ride6-rider.md`, "attach.json").

use anyhow::{Context, Result};
use blackbox_skeleton::Transform;
use glam::{Quat, Vec3};
use serde::Deserialize;

/// A rigid transform: rotation as a quaternion `x, y, z, w` and a translation.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Rigid {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}

impl Rigid {
    pub fn transform(&self) -> Transform {
        let [x, y, z, w] = self.rotation;
        Transform::from_rt(Quat::from_xyzw(x, y, z, w).normalize(), Vec3::from_array(self.translation))
    }
}

/// The two hubs in the bike skeleton's frame.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Hubs {
    pub front: [f32; 3],
    pub rear: [f32; 3],
}

/// A point a limb reaches for, in the bike skeleton's frame at rest.
#[derive(Debug, Clone, Deserialize)]
pub struct Target {
    pub name: String,
    pub position: [f32; 3],
    /// Its turn as a quaternion `x, y, z, w` in the same frame: the frame the hand or foot rests in. Files written before the
    /// turn was recorded have none.
    #[serde(default)]
    pub rotation: Option<[f32; 4]>,
    /// Whether it turns with the handlebar (a grip) or stays on the frame (a peg).
    pub steers: bool,
}

/// A limb solved with two-bone IK: the three bones, the bone its middle joint bends towards, the target it reaches for
/// and where its tip sits against that target in the authored pose (rider component space).
#[derive(Debug, Clone, Deserialize)]
pub struct Limb {
    pub name: String,
    pub root: String,
    pub mid: String,
    pub tip: String,
    pub pole: String,
    pub target: String,
    pub tip_offset: [f32; 3],
    /// Where the tip bone sits in the target's own frame (a hand against its grip, a foot against its peg): the target's frame
    /// composed with this is the tip's place and turn. Files written before the targets had a turn have none and use
    /// `tip_offset`.
    #[serde(default)]
    pub tip_frame: Option<Rigid>,
}

/// The bones the body leans forward with when the arms cannot reach the grips, and the bones that turn back to keep the head up.
#[derive(Debug, Clone, Deserialize)]
pub struct Torso {
    /// The spine, root to tip: the lean is split evenly over them, each turning about its own joint.
    pub lean: Vec<String>,
    /// The neck and head, turned back by `head_share` of the lean in all.
    pub head: Vec<String>,
    pub head_share: f32,
}

/// The rider's seat contact: the bones whose skin rests on the seat and how far it sinks into it.
#[derive(Debug, Clone, Deserialize)]
pub struct SeatRig {
    pub bones: Vec<String>,
    /// Metres the lowest skinned point sinks below the seat's top.
    pub sink: f32,
}

/// The rider's body shifting with the physics: his displacement from the rest place, held to a range and smoothed.
#[derive(Debug, Clone, Deserialize)]
pub struct Inertia {
    /// The rider bone that moves (it carries the legs, the spine and so the arms and head) and that his body's turning turns.
    pub bone: String,
    /// The range of the displacement in the car's axes (forward, left, up), metres: the animation's `InertiaDisplacement`
    /// input is held to it.
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// Unused since the shift follows the physics (it was a modelling choice for a body without one): files written before
    /// that still carry them.
    #[serde(default)]
    #[allow(dead_code)]
    pub braking_m: f32,
    #[serde(default)]
    #[allow(dead_code)]
    pub full_acceleration: f32,
    /// The rate (1/s) at which the shift follows.
    pub rate: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Attach {
    /// The rider's root bone in the bike skeleton's frame: the seat.
    pub rider_root_in_bike: Rigid,
    pub hubs: Hubs,
    pub targets: Vec<Target>,
    pub limbs: Vec<Limb>,
    pub inertia: Inertia,
    /// How the body bends to reach (`None`: it only shifts).
    #[serde(default)]
    pub torso: Option<Torso>,
    /// What rests on the seat (`None`: the rider is not sat on a surface).
    #[serde(default)]
    pub seat: Option<SeatRig>,
    /// The top of this bike's seat along its centre line, `[x, z]` samples in the bike skeleton's frame, x ascending (from the
    /// bike's `contacts.json`; empty: unknown).
    #[serde(default)]
    pub seat_surface: Vec<[f32; 2]>,
}

impl Attach {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).context("attach.json")
    }

    /// The index of the target named `name`.
    pub fn target_index(&self, name: &str) -> Option<usize> {
        self.targets.iter().position(|t| t.name == name)
    }

    /// How far the model's frame is from the bike skeleton's frame: the model's hubs minus the skeleton's, averaged over the
    /// two, with the bike on the centre line. The two frames have the same axes, so the move is all there is.
    pub fn frame_offset(&self, model_front_hub: Vec3, model_rear_hub: Vec3) -> Vec3 {
        let (front, rear) = (Vec3::from_array(self.hubs.front), Vec3::from_array(self.hubs.rear));
        let moved = ((model_front_hub - front) + (model_rear_hub - rear)) * 0.5;
        Vec3::new(moved.x, 0.0, moved.z)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A file as the converter writes it, with one limb.
    pub const SAMPLE: &str = r#"{
        "frame": "car", "reference_clip": "x",
        "rider_root_in_bike": {"translation": [-0.34, 0.0, 0.91], "rotation": [0.0, 0.0, 0.7071068, 0.7071068]},
        "hubs": {"front": [0.7356, 0.0, 0.2248], "rear": [-0.7389, 0.0, 0.2323]},
        "targets": [
            {"name": "left_grip", "position": [0.12, 0.33, 1.14], "steers": true},
            {"name": "left_foot", "position": [-0.26, 0.21, 0.42], "steers": false}
        ],
        "limbs": [
            {"name": "left_arm", "root": "upperarm_l", "mid": "lowerarm_l", "tip": "hand_l", "pole": "PV_arm_l",
             "target": "left_grip", "tip_offset": [0.01, 0.0, -0.02],
             "tip_frame": {"translation": [0.0, 0.0, 0.0], "rotation": [0.0, 0.0, 0.0, 1.0]}}
        ],
        "inputs": {"tuck_speed_kmh": 140.0},
        "inertia": {"bone": "pelvis", "min": [-0.06, -0.15, -0.04], "max": [0.0, 0.15, 0.2], "braking_m": 0.06,
                    "full_acceleration": 9.81, "rate": 4.0}
    }"#;

    #[test]
    fn a_file_as_the_converter_writes_it_reads_back() {
        let attach = Attach::parse(SAMPLE.as_bytes()).unwrap();
        assert_eq!(attach.targets.len(), 2);
        assert_eq!(attach.target_index("left_foot"), Some(1));
        assert_eq!(attach.target_index("nope"), None);
        assert!(attach.targets[0].steers && !attach.targets[1].steers);
        assert_eq!(attach.limbs[0].tip, "hand_l");
        assert_eq!(attach.inertia.bone, "pelvis");
        // A quarter turn about z turns +x into +y.
        let seat = attach.rider_root_in_bike.transform();
        assert!(seat.transform_vector(Vec3::X).distance(Vec3::Y) < 1e-5);
        assert!(seat.translation.distance(Vec3::new(-0.34, 0.0, 0.91)) < 1e-6);
    }

    #[test]
    fn a_file_missing_a_section_is_refused() {
        assert!(Attach::parse(b"{}").is_err());
        assert!(Attach::parse(b"not json").is_err());
    }

    #[test]
    fn the_model_frame_is_the_skeleton_frame_moved_so_the_front_hubs_agree() {
        let attach = Attach::parse(SAMPLE.as_bytes()).unwrap();
        let (front, rear) = (Vec3::new(0.7372, 0.0, 0.3045), Vec3::new(-0.7373, 0.0, 0.3118));
        let offset = attach.frame_offset(front, rear);
        assert!(offset.distance(Vec3::new(0.0016, 0.0, 0.0797)) < 1e-4, "{offset:?}");
        assert!((Vec3::from_array(attach.hubs.front) + offset).distance(front) < 1e-3);
        assert!((Vec3::from_array(attach.hubs.rear) + offset).distance(rear) < 1e-3);
        // A model whose hubs sit apart differently from the skeleton's splits the difference.
        let wide = attach.frame_offset(front + Vec3::X * 0.02, rear - Vec3::X * 0.02);
        assert!((wide.x - offset.x).abs() < 1e-6, "{wide:?}");
    }
}
