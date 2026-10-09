//! The tail-pipe effects of a car: where its pipes are, which particle group the nitrous and the gear-change
//! blow-off play, how long the blow-off lasts and whether the engine allows it
//! (docs/specs/exhaust-flames.md, sections 2 to 4).

use std::collections::HashMap;

use anyhow::{Context, Result};
use blackbox_attrib::{CollectionRef, Database};
use blackbox_hash::bstring_hash;
use blackbox_particles::EmitterSpec;
use blackbox_solid::Solid;
use blackbox_tpk::Texture;
use game_install::GameDir;
use glam::{Mat4, Vec3, Vec4};

use super::CarModel;
use super::assemble::Placement;
use crate::read_unwrapped;

/// The pack the particle textures are in.
const PARTICLE_PACK: &str = "GLOBAL/InGameB.bun";

/// The position markers of a tail pipe.
const PIPE_MARKERS: [&str; 3] = ["EXHAUST", "LEFT_EXHAUST", "RIGHT_EXHAUST"];

/// One emitter of a particle group and the texture (by name hash) its particles are drawn with.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupEmitter {
    pub spec: EmitterSpec,
    pub texture: u32,
}

/// What a car needs to show its exhaust effects.
#[derive(Debug, Clone, PartialEq)]
pub struct ExhaustFx {
    /// Each tail pipe's frame in the car model's space: the local `z` axis is the way the flame shoots.
    pub pipes: Vec<Mat4>,
    /// `ecar` `ShiftSpeed` and `ShiftAngle` (degrees per second, degrees); either 0 means no blow-off.
    pub shift_speed: f32,
    pub shift_angle: f32,
    /// `pvehicle` `engine_upgrades`: how many engine upgrade levels the car has.
    pub engine_upgrades: i32,
    /// The emitters of the `ecar` `NOSEffect` group.
    pub nitrous: Vec<GroupEmitter>,
}

impl ExhaustFx {
    /// Read the effects of `model` from the gameplay database (`attributes.bin`).
    pub fn read(db: &Database, model: &CarModel) -> Option<Self> {
        let key = model.car_type.as_deref().unwrap_or(&model.name).to_ascii_lowercase();
        let ecar = db.collection("ecar", &key)?;
        let engine_upgrades =
            db.collection("pvehicle", &key).and_then(|pvehicle| pvehicle.get_i32("engine_upgrades")).unwrap_or(0);
        let group = ecar.follow("NOSEffect");
        let nitrous = group.map(|g| group_emitters(db, g)).unwrap_or_default();
        Some(Self {
            pipes: pipe_frames(&model.placements, &model.solids),
            shift_speed: ecar.get_f32("ShiftSpeed").unwrap_or(0.0),
            shift_angle: ecar.get_f32("ShiftAngle").unwrap_or(0.0),
            engine_upgrades,
            nitrous,
        })
    }
}

/// The frame of every tail-pipe marker among the placed solids, in the car model's space.
pub fn pipe_frames(placements: &[Placement], solids: &HashMap<u32, Solid>) -> Vec<Mat4> {
    let names = PIPE_MARKERS.map(bstring_hash);
    let mut frames = Vec::new();
    for placement in placements {
        let Some(solid) = solids.get(&placement.solid) else { continue };
        for marker in solid.markers.iter().filter(|m| names.contains(&m.name_hash)) {
            // Stored row-major for row vectors: glam's column-major reading is the transpose.
            frames.push(placement.transform * Mat4::from_cols_array(&marker.matrix));
        }
    }
    frames
}

fn group_emitters(db: &Database, group: CollectionRef<'_>) -> Vec<GroupEmitter> {
    let Some(items) = group.get("Emitters").and_then(|v| v.as_array()) else { return Vec::new() };
    items
        .iter()
        .filter_map(|v| v.as_ref_spec())
        .filter_map(|r| db.resolve(r))
        .filter_map(|e| Some(GroupEmitter { spec: emitter_spec(e), texture: texture_hash(e)? }))
        .collect()
}

/// The name hash of the texture of an `emitterdata` collection (the first word of its texture record).
fn texture_hash(emitter: CollectionRef<'_>) -> Option<u32> {
    let bytes = emitter.get("Texture")?.as_raw()?;
    Some(u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?))
}

/// An `emitterdata` collection as emitter parameters. Fields the emitter library does not model (start delay,
/// cycles, one-shot, disc spread, texture animation) are ignored.
fn emitter_spec(e: CollectionRef<'_>) -> EmitterSpec {
    let float = |name: &str| e.get_f32(name).unwrap_or(0.0);
    let vector = |name: &str| e.get_vector4(name).map_or(Vec3::ZERO, |[x, y, z, _]| Vec3::new(x, y, z));
    let four = |name: &str| e.get_vector4(name).unwrap_or_default();
    let flag = |name: &str| e.get_i32(name).unwrap_or(0) != 0;
    let color = |name: &str| e.get_u32(name).unwrap_or(0).to_be_bytes();
    let keys = four("KeyPositions").map(|k| k / 3.0);
    let no_kill = e.get_bool("NoKillAtAlpha").unwrap_or(false);
    EmitterSpec {
        rate: float("NumParticles"),
        rate_variance: float("NumParticlesVariance"),
        life: float("Life"),
        life_variance: float("LifeVariance"),
        speed: float("Speed"),
        speed_variance: float("SpeedVariance"),
        spread: float("SpreadAngle"),
        velocity_start: vector("VelocityStart"),
        velocity_delta: vector("VelocityDelta"),
        accel_start: vector("AccelStart"),
        accel_delta: vector("AccelDelta"),
        world_axis_velocity: e.get_bool("EliminateUnnecessaryRandomness").unwrap_or(false),
        volume_center: vector("VolumeCenter"),
        volume_extent: vector("VolumeExtent"),
        drag: float("Drag"),
        gravity: float("Gravity"),
        inherit: float("MotionInherit"),
        inherit_variance: float("MotionInheritVariance"),
        live_motion: flag("MotionLive"),
        initial_angle_range: float("InitialAngleRange"),
        rotation_variance: float("RotationVariance"),
        random_rotation_direction: flag("RandomRotationDirection"),
        keys,
        size: four("Size"),
        angle: four("RelativeAngle"),
        colors: [color("Color1"), color("Color2"), color("Color3"), color("Color4")],
        kill_alpha: match no_kill {
            true => None,
            false => Some(e.get_i32("AlphaToKillAt").unwrap_or(0).clamp(0, 255) as u8),
        },
    }
}

/// The textures of the particle pack with these name hashes. The blend is in each texture's `alpha_blend`
/// (2 is additive, 1 alpha blending).
pub fn particle_textures(dir: &GameDir, hashes: &[u32]) -> Result<Vec<Texture>> {
    let file = read_unwrapped(dir, PARTICLE_PACK)?;
    let packs = blackbox_tpk::read_texture_packs(&file).with_context(|| format!("texture packs in {PARTICLE_PACK}"))?;
    let mut found: Vec<Texture> = Vec::new();
    for texture in packs.into_iter().flat_map(|p| p.textures) {
        if hashes.contains(&texture.name_hash) && !found.iter().any(|t| t.name_hash == texture.name_hash) {
            found.push(texture);
        }
    }
    Ok(found)
}

/// Texture `alpha_blend` value of an additive particle texture.
pub const ADDITIVE_BLEND: u8 = 2;

/// The direction a pipe shoots along, in the car model's space.
pub fn pipe_direction(frame: &Mat4) -> Vec3 {
    let z: Vec4 = frame.z_axis;
    z.truncate()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_with_markers(name: &str, markers: &[(&str, [f32; 3])]) -> Solid {
        Solid {
            name: name.into(),
            name_hash: bstring_hash(name),
            version: 0,
            flags: 0,
            bounds_min: [0.0; 3],
            bounds_max: [0.0; 3],
            transform: [0.0; 16],
            num_polys: 0,
            density: 0.0,
            texture_hashes: Vec::new(),
            light_material_hashes: Vec::new(),
            markers: markers
                .iter()
                .map(|(n, at)| {
                    let mut matrix = Mat4::IDENTITY.to_cols_array();
                    matrix[8..11].copy_from_slice(&[-1.0, 0.0, 0.0]);
                    matrix[12..15].copy_from_slice(at);
                    blackbox_solid::PositionMarker {
                        name_hash: bstring_hash(n),
                        int_param: 0,
                        float_params: [0.0; 2],
                        matrix,
                    }
                })
                .collect(),
            vertex_buffers: Vec::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            groups: Vec::new(),
        }
    }

    fn placement(solid: &Solid, transform: Mat4) -> Placement {
        Placement { solid: solid.name_hash, transform, left_brake: false, slot: 0, corner: None }
    }

    #[test]
    fn every_exhaust_marker_of_every_placed_solid_is_a_pipe() {
        let body = solid_with_markers(
            "BODY",
            &[
                ("LEFT_EXHAUST", [-2.0, 0.3, 0.1]),
                ("RIGHT_EXHAUST", [-2.0, -0.3, 0.1]),
                ("LEFT_HEADLIGHT", [2.0, 0.5, 0.5]),
            ],
        );
        let bumper = solid_with_markers("BUMPER", &[("EXHAUST", [0.0, 0.0, 0.0])]);
        let placements =
            [placement(&body, Mat4::IDENTITY), placement(&bumper, Mat4::from_translation(Vec3::new(-2.5, 0.0, 0.2)))];
        let solids = HashMap::from([(body.name_hash, body), (bumper.name_hash, bumper)]);
        let frames = pipe_frames(&placements, &solids);
        assert_eq!(frames.len(), 3);
        assert!((frames[0].w_axis.truncate() - Vec3::new(-2.0, 0.3, 0.1)).length() < 1e-6);
        assert!((frames[2].w_axis.truncate() - Vec3::new(-2.5, 0.0, 0.2)).length() < 1e-6);
        // The marker's z row points backwards.
        assert!((pipe_direction(&frames[0]) - Vec3::NEG_X).length() < 1e-6);
    }

    #[test]
    fn a_solid_that_is_not_placed_adds_no_pipe() {
        let body = solid_with_markers("BODY", &[("EXHAUST", [0.0; 3])]);
        let frames = pipe_frames(&[], &HashMap::from([(body.name_hash, body)]));
        assert!(frames.is_empty());
    }
}
