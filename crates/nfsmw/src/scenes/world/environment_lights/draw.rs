//! Shared car/environment optical rules with authored environment admission.

use std::collections::HashMap;

use blackbox_gfx::{BlendMode, EffectLayer, TextureHandle, TexturedEffect};
use glam::{Mat4, Vec3};
use nfsmw_data::world::lights::Marker;

use super::super::car_light_flares::{FlareView, appearance, focal_scale};
use super::EnvironmentLights;

/// The original submits at most 50 scenery markers per frame, before blink gating.
const POOL_SIZE: usize = 50;

pub(super) fn sample(marker: Marker, world: Mat4, camera: Vec3, forward: Vec3) -> Option<(Vec3, f32)> {
    let at = world.transform_point3(marker.position);
    let to_eye = camera - at;
    if !at.is_finite() || !to_eye.is_finite() || to_eye.length_squared() < f32::EPSILON {
        return None;
    }
    if marker.kind.directional() {
        let delta = at - world.w_axis.truncate();
        let direction = Vec3::new(delta.x, delta.y, 0.0).normalize_or_zero();
        return Some((at, direction.dot(to_eye.normalize_or_zero())));
    }
    let cosine = (-to_eye).normalize_or_zero().dot(forward);
    let gain = ((cosine - 0.96) / 0.04).clamp(0.0, 1.0);
    (gain > 0.0).then_some((at, gain))
}

impl EnvironmentLights {
    pub(super) fn draw(
        &self,
        materials: &HashMap<u32, (TextureHandle, BlendMode)>,
        view: FlareView,
        out: &mut Vec<TexturedEffect>,
    ) {
        let Some(focal) = focal_scale(view.height, view.fov_degrees) else { return };
        let right = view.forward.cross(Vec3::Z).normalize_or(Vec3::Y);
        let up = right.cross(view.forward).normalize_or(Vec3::Z);
        // One batch per installed profile, rather than one GPU upload per marker.
        for (kind, profile) in &self.profiles {
            let Some(&(texture, _)) = materials.get(&profile.texture) else { continue };
            let mut vertices = Vec::new();
            let mut allocated = 0;
            'instances: for instance in &self.visible {
                let Some(markers) = self.sources.get(&instance.mesh) else { continue };
                for &marker in markers.iter() {
                    if allocated == POOL_SIZE {
                        break 'instances;
                    }
                    allocated += 1;
                    if marker.kind != *kind {
                        continue;
                    }
                    let Some((at, gain)) = sample(marker, instance.transform, view.camera, view.forward) else {
                        continue;
                    };
                    if !self.blink.lit(*kind, at.x) {
                        continue;
                    }
                    let to_eye = view.camera - at;
                    let depth = (-to_eye).dot(view.forward);
                    if depth <= 0.3 || !depth.is_finite() {
                        continue;
                    }
                    let (intensity, facing) = match kind.directional() {
                        true => (1.0, gain),
                        false => (gain, 1.0),
                    };
                    let Some((radius, color)) =
                        appearance(profile, intensity, facing, depth / focal, to_eye.length_squared())
                    else {
                        continue;
                    };
                    let at = at + to_eye.normalize_or_zero() * profile.z_bias;
                    let x = right * radius;
                    let y = up * radius;
                    EffectLayer::particle_quad(
                        &mut vertices,
                        [at - x + y, at + x + y, at + x - y, at - x - y],
                        color,
                        [0.0; 2],
                    );
                }
            }
            if !vertices.is_empty() {
                out.push(TexturedEffect { texture, blend: BlendMode::Additive, vertices });
            }
        }
    }
}
