//! The rider's ragdoll: the bodies and joints of the RIDE 6 rider's physics asset and the way a fall throws him, converted to the
//! `ragdoll.json` the game's rider reads (`docs/formats/ride6-rider.md`, "ragdoll.json"; `docs/specs/ride6-rider-fall.md`).
//!
//! Input: two JSON files the user's own tooling writes from the install (the `json` mode of the Unreal package reader), the
//! rider's physics asset (`RiderGP_PhysicsAsset`) and the rider blueprint (`RiderBP`, whose ragdoll component holds the
//! configuration of each kind of fall). Output: `ragdoll.json`, in `files/RIDER/` of the mod (never committed). Unreal's frame is
//! left-handed with x forward, y right, z up and its lengths are centimetres; the file is in the rider's frame (x forward, y
//! left, z up) and metres, in the frames of the bones the bodies are on ([`crate::rider::convert`]).

mod asset;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use asset::{Body, Joint};

/// The friction of the rider's physical material against the road: `PM_RiderRagdoll_CB` (`Friction`, `StaticFriction` 0.62; it
/// combines with the road's by averaging, the material's default).
pub const RIDER_FRICTION: f32 = 0.62;
/// The friction of the road: `PM_T_Tarmac` sets none, so the material's default, 0.7.
pub const TARMAC_FRICTION: f32 = 0.7;
/// The linear damping of a body of the asset: the engine's default (`FBodyInstance::LinearDamping`, 0.01); the angular one is 0.
pub const LINEAR_DAMPING: f32 = 0.01;
/// The factor on the velocity the rider leaves with: `RagdollVelocityMultiplier`, 1, the default of the rider component's
/// constructor (`UIgnitionRiderComponent`, offset 0x484; no blueprint changes it).
pub const VELOCITY_MULTIPLIER: f32 = 1.0;
/// How fast a joint's turning dies away (1/s). A modelling choice: the asset's drives (position and velocity springs of the
/// physical animation) are not modelled, and a limp limb must not swing for ever.
pub const JOINT_DAMPING: f32 = 4.0;
/// Km/h to m/s: the engine's fall configurations are in km/h (the engine turns them to cm/s with a factor of 100/3.6).
const KMH: f32 = 1.0 / 3.6;
/// The version the game checks.
pub const VERSION: u32 = 1;

/// What one run wrote.
#[derive(Debug)]
pub struct Summary {
    pub bodies: usize,
    pub joints: usize,
}

/// A number written with six decimals: the file is read by people too.
fn round(value: f32) -> f64 {
    (f64::from(value) * 1e6).round() / 1e6
}

/// A list of numbers, rounded.
fn vector<const N: usize>(values: [f32; N]) -> Value {
    Value::Array(values.iter().map(|&v| json!(round(v))).collect())
}

/// The `ragdoll.json` of a rider from his physics asset and his blueprint (both the JSON of the package reader).
pub fn convert(physics_asset: &Value, rider_blueprint: &Value) -> Result<Value> {
    let bodies = asset::bodies(physics_asset)?;
    let joints = asset::joints(physics_asset, &bodies)?;
    let falls = asset::falls(rider_blueprint)?;
    let body = |b: &Body| {
        json!({
            "bone": b.bone, "mass": round(b.mass), "radius": round(b.radius), "half_length": round(b.half_length),
            "centre": vector(b.centre.to_array()), "axis": vector(b.axis.to_array()),
        })
    };
    let joint = |j: &Joint| {
        json!({
            "child": j.child, "parent": j.parent,
            "frame_child": vector(j.frame_child.to_array()), "frame_parent": vector(j.frame_parent.to_array()),
            "swing_z_deg": round(j.swing_z), "swing_y_deg": round(j.swing_y), "twist_deg": round(j.twist),
        })
    };
    let fall = |f: &asset::Fall| {
        json!({
            "additional": vector((f.additional * KMH).to_array()), "scale": vector(f.scale.to_array()),
            "world": vector((f.world * KMH).to_array()), "angular": round(f.angular),
        })
    };
    Ok(json!({
        "version": VERSION,
        "bodies": bodies.iter().map(body).collect::<Vec<_>>(),
        "joints": joints.iter().map(joint).collect::<Vec<_>>(),
        "contact_friction": (RIDER_FRICTION + TARMAC_FRICTION) / 2.0,
        "linear_damping": LINEAR_DAMPING,
        "angular_damping": 0.0,
        "joint_damping": JOINT_DAMPING,
        "falls": {
            "velocity_multiplier": VELOCITY_MULTIPLIER,
            "high_side": fall(&falls.high_side),
            "low_side": fall(&falls.low_side),
        },
    }))
}

/// Reads the two JSON files and writes `ragdoll.json` into `output`.
pub fn import(physics_asset: &Path, rider_blueprint: &Path, output: &Path) -> Result<Summary> {
    let read = |path: &Path| -> Result<Value> {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_slice(&bytes).with_context(|| path.display().to_string())
    };
    let file = convert(&read(physics_asset)?, &read(rider_blueprint)?)?;
    fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    let text = serde_json::to_string_pretty(&file)?;
    fs::write(output.join("ragdoll.json"), text).context("writing ragdoll.json")?;
    let count = |key: &str| file[key].as_array().map_or(0, Vec::len);
    Ok(Summary { bodies: count("bodies"), joints: count("joints") })
}

#[cfg(test)]
mod tests;
