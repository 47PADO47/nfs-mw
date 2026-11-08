//! Where each grip's rubber lies along its axis, from the bike's exported chassis mesh, written to `contacts.json` as `grip_span` so the
//! game can centre the glove on it (`docs/specs/ride6-rider-hands.md`, section 3).
//!
//! The hand is held by the grip bone, whose origin lies on the grip's axis at the rubber's **inboard** end (measured on twelve bikes:
//! the vertices skinned to `BONE_{L,R}_GRIP` run from 0 to 11 to 14 cm outboard of it, within 1 cm inboard on a scooter). The span is
//! `[inboard, outboard]` metres along the grip, outboard positive: the vertices skinned mostly to the grip bone, projected on the
//! bone's own y axis (the exporter's z, the grip's cylinder; `docs/formats/ride6-rider.md`).

use std::path::Path;

use anyhow::Result;
use glam::{Quat, Vec3};
use serde_json::{Value, json};

use crate::exports::Exports;
use crate::partfile::{Kind, PartTable, Role};

/// The fewest vertices a grip must have for its span to mean anything.
const MIN_VERTICES: usize = 8;

/// The `[inboard, outboard]` span of the grip held on the bone `bone` of the transform (`at`, `turn`), `outboard` being the bone's
/// local y times `sign`; `points` are `(joint name, position in the car frame)` of the mesh's vertices that the joint moves most.
pub fn span<'a>(
    points: impl Iterator<Item = (&'a str, Vec3)>,
    bone: &str,
    at: Vec3,
    turn: Quat,
    sign: f32,
) -> Option<[f32; 2]> {
    let axis = turn * Vec3::Y * sign;
    let along: Vec<f32> = points.filter(|(name, _)| *name == bone).map(|(_, p)| (p - at).dot(axis)).collect();
    if along.len() < MIN_VERTICES {
        return None;
    }
    let low = along.iter().copied().fold(f32::INFINITY, f32::min);
    let high = along.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    Some([low, high])
}

fn vec(value: Option<&Value>) -> Option<Vec<f32>> {
    value?.as_array()?.iter().map(|v| v.as_f64().map(|f| f as f32)).collect()
}

/// Adds `grip_span` to each grip target of `contacts` that has its bone's place and turn, measured on the chassis mesh in `bike_dir`.
/// A folder without meshes, or a grip without enough vertices, leaves the target as it is.
pub fn add(contacts: &mut Value, bike_dir: &Path) -> Result<()> {
    if !bike_dir.join("glb").is_dir() {
        return Ok(());
    }
    let table = PartTable::read(bike_dir)?;
    let exports = Exports::scan(&bike_dir.join("glb"))?;
    let Some(targets) = contacts.get_mut("targets").and_then(Value::as_array_mut) else { return Ok(()) };
    for (role, bone) in super::attach::GRIP_BONES {
        let Some(target) = targets.iter_mut().find(|t| t.get("name").and_then(Value::as_str) == Some(role)) else {
            continue;
        };
        let (Some(at), Some(q)) = (vec(target.get("anchor")), vec(target.get("anchor_rotation"))) else { continue };
        if at.len() != 3 || q.len() != 4 {
            continue;
        }
        let (at, turn) = (Vec3::new(at[0], at[1], at[2]), Quat::from_xyzw(q[0], q[1], q[2], q[3]).normalize());
        let sign = if role.starts_with("left") { 1.0 } else { -1.0 };
        let mut points: Vec<(String, Vec3)> = Vec::new();
        for part in table.parts.iter().filter(|p| p.kind == Kind::Body && p.role == Role::Body) {
            let glb = exports.load(&part.export, 0)?;
            for primitive in glb.primitives.iter().filter(|p| p.is_skinned()) {
                for (i, position) in primitive.positions.iter().enumerate() {
                    let weights = primitive.weights[i];
                    let strongest = (0..4).fold(0, |best, k| if weights[k] > weights[best] { k } else { best });
                    let joint = usize::from(primitive.joints[i][strongest]);
                    let Some(node) = glb.joints.get(joint).and_then(|n| glb.nodes.get(*n)) else { continue };
                    points.push((node.name.clone(), super::seat_surface::car_point(&part.anchor, *position)));
                }
            }
        }
        let found = span(points.iter().map(|(name, p)| (name.as_str(), *p)), bone, at, turn, sign);
        if let Some([low, high]) = found {
            target["grip_span"] = json!([round(low), round(high)]);
        }
    }
    Ok(())
}

/// Millimetres, so the file stays short.
fn round(metres: f32) -> f32 {
    (metres * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_span_runs_along_the_bones_own_y_in_the_direction_of_the_sign() {
        // A right grip: the bone's y points inboard (the car's +y), the rubber runs 11 cm outboard (to -y) of the bone.
        let at = Vec3::new(0.2, -0.29, 1.1);
        let points: Vec<(&str, Vec3)> = (0..12)
            .map(|k| ("BONE_R_GRIP", at + Vec3::new(0.0, -0.01 * k as f32, 0.0)))
            .chain([("BONE_HANDLE", at)])
            .collect();
        let [low, high] = span(points.iter().copied(), "BONE_R_GRIP", at, Quat::IDENTITY, -1.0).unwrap();
        assert!(low.abs() < 1e-6 && (high - 0.11).abs() < 1e-6, "{low} {high}");
        // The bone turned a half turn about x has its y the other way: the same grip is measured along the opposite sign.
        let turned = Quat::from_rotation_x(std::f32::consts::PI);
        assert_eq!(
            span(points.iter().copied(), "BONE_R_GRIP", at, turned, 1.0).map(|s| (s[1] * 1000.0).round()),
            Some(110.0)
        );
        // Too few vertices, or none for the bone: no span.
        assert_eq!(span(points.iter().take(4).copied(), "BONE_R_GRIP", at, Quat::IDENTITY, -1.0), None);
        assert_eq!(span(points.iter().copied(), "BONE_L_GRIP", at, Quat::IDENTITY, 1.0), None);
    }

    #[test]
    fn a_folder_without_meshes_is_left_alone() {
        let mut contacts = json!({"targets": [{"name": "right_grip"}]});
        add(&mut contacts, Path::new("no-such-folder")).unwrap();
        assert_eq!(contacts, json!({"targets": [{"name": "right_grip"}]}));
    }
}
