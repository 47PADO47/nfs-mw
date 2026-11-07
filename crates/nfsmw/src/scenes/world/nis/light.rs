//! The light of a NIS character (`docs/specs/nis-player-2.md`, section 9.6) [decomp]. The original lights its skinned
//! characters with a rig of their own: two lights fixed to the camera, the same at any time of day, with no ambient.
//! Per vertex, each light's cosine is wrapped by [`WRAP`] and clamped, the summed light is scaled by the light
//! material's diffuse ramp over how much the surface faces the eye, and the result is saturated. The rewrite works it
//! out on the CPU when it skins the parts and puts it in the vertex colour, which `Shading::Character` multiplies with
//! the texture.

use std::collections::HashMap;

use blackbox_solid::Vertex;
use glam::{Mat4, Vec3, Vec4};
use nfsmw_data::car::LightMaterial;

use crate::scenes::car::original_lighting::spherical;

/// The rig's two lights, a fill and a key: theta and phi in degrees, RGB and intensity. Both are camera-space entries
/// (section 9.6) [decomp].
const RIG: [(f32, f32, [f32; 3], f32); 2] = [(122.9, 194.0, [0.5, 0.5, 0.5], 0.53), (28.0, 0.0, [1.0, 1.0, 1.0], 0.59)];
/// Added to each light's cosine before it is clamped to 0..1 (section 9.6) [decomp].
const WRAP: f32 = 0.3;
/// The facing term never falls below this (section 9.6) [decomp].
const MIN_FACING: f32 = 0.01;
/// The diffuse alpha range is this share of the authored alpha span (`car-lighting.md`, "Material constants") [decomp].
const ALPHA_RANGE: f32 = 0.25;

/// The camera a character is lit for, in render space: where it is, and its right, down and forward axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eye {
    pub position: Vec3,
    pub right: Vec3,
    pub down: Vec3,
    pub forward: Vec3,
}

impl Eye {
    /// The eye of a camera at `position` whose world-to-view matrix is `view` (right-handed: x right, y up, looking
    /// down -z).
    pub fn new(view: Mat4, position: Vec3) -> Self {
        let row = |i: usize| view.row(i).truncate();
        Self { position, right: row(0), down: -row(1), forward: -row(2) }
    }
}

/// A directional light: the unit direction toward it and its colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub toward: Vec3,
    pub colour: Vec3,
}

/// The rig's lights for `eye`, in render space. An entry's spherical direction `v` (azimuth theta + 180, polar angle
/// 90 - phi) is the camera-space direction `(-v.y, -v.z, v.x)` along the camera's right, down and forward axes. The
/// colour is the RGB times the intensity.
pub fn rig(eye: &Eye) -> [Light; 2] {
    RIG.map(|(theta, phi, rgb, intensity)| {
        let v = spherical(theta + 180.0, 90.0 - phi);
        let toward = eye.right * -v.y + eye.down * -v.z + eye.forward * v.x;
        Light { toward: toward.normalize_or(Vec3::Z), colour: Vec3::from_array(rgb) * intensity }
    })
}

/// A light material's diffuse ramp: the RGBA where the surface is seen edge-on, and what is added where it faces the
/// eye.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ramp {
    pub min: Vec4,
    pub range: Vec4,
}

impl Ramp {
    /// A ramp that neither brightens nor darkens, for a group whose light material the table lacks [ours].
    pub const NEUTRAL: Self = Self { min: Vec4::ONE, range: Vec4::ZERO };

    /// The diffuse ramp of `material`: the minimum is MinScale times MinRGB with MinAlpha, the range MaxScale times MaxRGB
    /// less the minimum, with [`ALPHA_RANGE`] of MaxAlpha less MinAlpha (`car-lighting.md`, "Material constants").
    pub fn of(material: &LightMaterial) -> Self {
        let min = Vec3::from_array(material.diffuse_min) * material.diffuse_min_scale;
        let max = Vec3::from_array(material.diffuse_max) * material.diffuse_max_scale;
        let alpha_range = (material.diffuse_max_alpha - material.diffuse_min_alpha) * ALPHA_RANGE;
        Self { min: min.extend(material.diffuse_min_alpha), range: (max - min).extend(alpha_range) }
    }
}

/// The diffuse ramps of the light materials `materials`, by name hash.
pub fn ramps(materials: &[LightMaterial]) -> HashMap<u32, Ramp> {
    materials.iter().map(|material| (material.name_hash, Ramp::of(material))).collect()
}

/// The rig and the eye in one actor's own frame, where its bind-pose vertices are: the original lights the unskinned
/// position and normal, so the bones move a vertex but not its light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lighting {
    eye: Vec3,
    lights: [Light; 2],
}

impl Lighting {
    /// The lighting of an actor whose bind-pose vertices `frame` takes to render space, seen from `eye`.
    pub fn new(eye: &Eye, frame: Mat4) -> Self {
        let to_local = frame.inverse();
        let lights = rig(eye).map(|light| Light {
            toward: to_local.transform_vector3(light.toward).normalize_or(light.toward),
            colour: light.colour,
        });
        Self { eye: to_local.transform_point3(eye.position), lights }
    }

    /// The D3DCOLOR bytes (B, G, R, A) of the bind-pose vertex `vertex`, whose shading group has the ramp `ramp`.
    pub fn colour(&self, vertex: &Vertex, ramp: &Ramp) -> [u8; 4] {
        let (normal, position) = (Vec3::from_array(vertex.normal), Vec3::from_array(vertex.position));
        let light = vertex_light(normal, position, self.eye, &self.lights, ramp);
        let byte = |x: f32| (x * 255.0).round() as u8;
        [byte(light.z), byte(light.y), byte(light.x), byte(light.w)]
    }
}

/// The light of a vertex at `position` with the unit normal `normal`, seen from `eye`, all in one frame: RGBA in 0..1.
/// The facing term is the cosine toward the eye clamped to [`MIN_FACING`]..1, and the ramp gives `min + range *
/// facing`. Each light adds its colour times its cosine plus [`WRAP`], clamped to 0..1. RGB is the ramp times the summed
/// light and alpha the ramp's own, both saturated as the original's vertex colour output is.
pub fn vertex_light(normal: Vec3, position: Vec3, eye: Vec3, lights: &[Light; 2], ramp: &Ramp) -> Vec4 {
    let facing = normal.dot((eye - position).normalize_or_zero()).clamp(MIN_FACING, 1.0);
    let diffuse = ramp.min + ramp.range * facing;
    let light = lights
        .iter()
        .fold(Vec3::ZERO, |sum, light| sum + light.colour * (normal.dot(light.toward) + WRAP).clamp(0.0, 1.0));
    (diffuse.truncate() * light).extend(diffuse.w).clamp(Vec4::ZERO, Vec4::ONE)
}

#[cfg(test)]
#[path = "light_tests.rs"]
mod tests;
