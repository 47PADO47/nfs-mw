//! `ragdoll.json`: the rider's body as a ragdoll, written by a mod's converter and read here without knowing what game the rider
//! comes from (`docs/formats/ride6-rider.md`, "ragdoll.json"; `docs/specs/ride6-rider-fall.md`).
//!
//! Everything is in the rider's own frame (x forward, y left, z up), metres, kilograms and seconds, and in the frames of the
//! rider's bones: a body is a capsule on a bone, a joint holds a child bone's body to a parent bone's with limits in the joint's
//! constraint frame. A rider without the file keeps to his bike when it falls.

use anyhow::{Context, Result, bail};
use glam::{Quat, Vec3};
use serde::Deserialize;

/// A limit of 180 degrees or more leaves the axis free.
pub const FREE_DEGREES: f32 = 180.0;

/// A capsule on a bone.
#[derive(Debug, Clone, Deserialize)]
pub struct BodyFile {
    pub bone: String,
    pub mass: f32,
    pub radius: f32,
    /// Half the distance between the centres of the two end spheres.
    pub half_length: f32,
    /// The capsule's centre in the bone's frame, and the direction of its axis there.
    pub centre: [f32; 3],
    pub axis: [f32; 3],
}

/// A joint between the bodies of two bones: the child's bone origin is the shared point; the frames are the constraint frames in
/// the two bones' frames (a quaternion `x, y, z, w`), with the twist about their X axis; the limits are in degrees.
#[derive(Debug, Clone, Deserialize)]
pub struct JointFile {
    pub child: String,
    pub parent: String,
    pub frame_child: [f32; 4],
    pub frame_parent: [f32; 4],
    pub swing_z_deg: f32,
    pub swing_y_deg: f32,
    pub twist_deg: f32,
}

/// How a fall of one kind changes the velocity the rider leaves with, in the rider's frame (m/s): the velocity `v` becomes
/// `v + additional + R (scale * (R^-1 v)) + world`, and his angular velocity is multiplied by `angular`.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct FallConfig {
    pub additional: [f32; 3],
    pub scale: [f32; 3],
    pub world: [f32; 3],
    pub angular: f32,
}

/// The fall kinds that change anything; any other fall leaves with the velocity the bike had and no spin.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Falls {
    /// The factor on the velocity the rider leaves with, before the configuration.
    pub velocity_multiplier: f32,
    pub high_side: FallConfig,
    pub low_side: FallConfig,
}

/// The whole file.
#[derive(Debug, Clone, Deserialize)]
pub struct RagdollFile {
    pub bodies: Vec<BodyFile>,
    pub joints: Vec<JointFile>,
    /// The friction coefficient of his body against the road.
    pub contact_friction: f32,
    #[serde(default)]
    pub linear_damping: f32,
    #[serde(default)]
    pub angular_damping: f32,
    /// The rate (1/s) at which a joint's turning dies away.
    #[serde(default)]
    pub joint_damping: f32,
    pub falls: Falls,
}

impl RagdollFile {
    /// Parse and check the file: positive masses and sizes, finite numbers, joints between the bodies' bones.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let file: Self = serde_json::from_slice(bytes).context("ragdoll.json")?;
        if file.bodies.is_empty() {
            bail!("ragdoll.json has no bodies");
        }
        for body in &file.bodies {
            let numbers = [body.mass, body.radius, body.half_length];
            if numbers.iter().any(|n| !n.is_finite() || *n < 0.0) || body.mass <= 0.0 || body.radius <= 0.0 {
                bail!("the body of {} has no mass or size", body.bone);
            }
            if Vec3::from_array(body.axis).length() < 1e-3 || !Vec3::from_array(body.centre).is_finite() {
                bail!("the body of {} has no axis", body.bone);
            }
        }
        let has = |bone: &str| file.bodies.iter().any(|b| b.bone == bone);
        for joint in &file.joints {
            if !has(&joint.child) || !has(&joint.parent) || joint.child == joint.parent {
                bail!("the joint {} > {} joins bones that have no body", joint.parent, joint.child);
            }
            let limits = [joint.swing_z_deg, joint.swing_y_deg, joint.twist_deg];
            if limits.iter().any(|l| !l.is_finite() || *l < 0.0) {
                bail!("the joint {} > {} has a bad limit", joint.parent, joint.child);
            }
            if frame(joint.frame_child).is_none() || frame(joint.frame_parent).is_none() {
                bail!("the joint {} > {} has a bad frame", joint.parent, joint.child);
            }
        }
        Ok(file)
    }
}

/// A frame as a unit quaternion; `None` if it has no direction.
pub fn frame(q: [f32; 4]) -> Option<Quat> {
    let q = Quat::from_xyzw(q[0], q[1], q[2], q[3]);
    (q.is_finite() && q.length() > 1e-3).then(|| q.normalize())
}

/// A limit in degrees as radians (pi for a free axis).
pub fn limit(degrees: f32) -> f32 {
    degrees.min(FREE_DEGREES).to_radians()
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const SAMPLE: &str = r#"{
        "bodies": [
            {"bone": "pelvis", "mass": 7.0, "radius": 0.11, "half_length": 0.075, "centre": [0.0, 0.0, 0.0], "axis": [0.0, 0.0, 1.0]},
            {"bone": "thigh_l", "mass": 7.0, "radius": 0.07, "half_length": 0.125, "centre": [0.25, 0.0, 0.0], "axis": [1.0, 0.0, 0.0]}
        ],
        "joints": [
            {"child": "thigh_l", "parent": "pelvis", "frame_child": [0, 0, 0, 1], "frame_parent": [0, 0, 0, 1],
             "swing_z_deg": 50, "swing_y_deg": 5, "twist_deg": 5}
        ],
        "contact_friction": 0.66,
        "falls": {"velocity_multiplier": 1.0,
                  "high_side": {"additional": [0, 0, 1.6], "scale": [0.05, 0, -0.28], "world": [0, 0, 0], "angular": 0.5},
                  "low_side": {"additional": [0, 0, 0], "scale": [0.05, 0, 0.32], "world": [0, 0, 0], "angular": 0.5}}
    }"#;

    #[test]
    fn a_file_parses_and_its_numbers_are_kept() {
        let file = RagdollFile::parse(SAMPLE.as_bytes()).unwrap();
        assert_eq!(file.bodies.len(), 2);
        assert_eq!(file.joints[0].child, "thigh_l");
        assert_eq!(file.falls.high_side.additional, [0.0, 0.0, 1.6]);
        assert_eq!(file.falls.low_side.angular, 0.5);
        assert_eq!(file.linear_damping, 0.0, "the damping defaults");
        assert!((limit(50.0) - 50.0_f32.to_radians()).abs() < 1e-6);
        assert_eq!(limit(180.0), std::f32::consts::PI);
        assert_eq!(limit(400.0), std::f32::consts::PI);
    }

    #[test]
    fn a_file_with_nothing_to_simulate_or_a_joint_between_strangers_is_refused() {
        assert!(RagdollFile::parse(b"not json").is_err());
        let none = SAMPLE.replace(r#""bodies": ["#, r#""bodies": [], "x": ["#);
        assert!(RagdollFile::parse(none.as_bytes()).is_err());
        let stranger = SAMPLE.replace(r#""parent": "pelvis""#, r#""parent": "elbow""#);
        assert!(RagdollFile::parse(stranger.as_bytes()).unwrap_err().to_string().contains("no body"));
        let massless = SAMPLE.replacen(r#""mass": 7.0"#, r#""mass": 0.0"#, 1);
        assert!(RagdollFile::parse(massless.as_bytes()).unwrap_err().to_string().contains("pelvis"));
        let bent = SAMPLE.replace(r#""frame_child": [0, 0, 0, 1]"#, r#""frame_child": [0, 0, 0, 0]"#);
        assert!(RagdollFile::parse(bent.as_bytes()).unwrap_err().to_string().contains("frame"));
        let limit = SAMPLE.replace(r#""twist_deg": 5"#, r#""twist_deg": -5"#);
        assert!(RagdollFile::parse(limit.as_bytes()).is_err());
    }
}
