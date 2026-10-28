//! Vehicle lens flares from installed profiles and marker directions.

use blackbox_gfx::{BlendMode, EffectLayer, RenderBackend, TextureHandle, TexturedEffect};
use glam::{Mat4, Vec3};
use nfsmw_data::car::{CarModel, lights::FlareProfile};

use super::car_lights::{CarLights, Kind, LampState};
pub(super) mod emergency;
mod enhanced;
mod intensity;
mod mod_lamps;
mod trails;

#[derive(Clone)]
struct Profile {
    kind: Kind,
    spec: FlareProfile,
    texture: TextureHandle,
}

#[derive(Clone, Default)]
pub struct CarLightFlares {
    profiles: Vec<Profile>,
    textures: Vec<TextureHandle>,
    enhanced_texture: Option<TextureHandle>,
    /// The sprites a mod car describes itself; empty for every other car.
    mod_lamps: mod_lamps::ModLamps,
}

#[derive(Clone, Copy)]
pub struct FlareView {
    pub camera: Vec3,
    pub forward: Vec3,
    pub fov_degrees: f32,
    pub height: u32,
}

impl CarLightFlares {
    pub fn upload(renderer: &mut dyn RenderBackend, model: &CarModel) -> Self {
        let mut set = Self::default();
        let mut uploaded = std::collections::HashMap::new();
        let brakes = match model.driving_lods.traffic {
            true => &model.flare_profiles.traffic_brake,
            false => &model.flare_profiles.brake,
        };
        for (kind, profiles) in [
            (Kind::Head, &model.flare_profiles.head),
            (Kind::Tail, brakes),
            (Kind::Reverse, &model.flare_profiles.reverse),
            (Kind::Siren(true), &model.flare_profiles.cop_red),
            (Kind::Siren(false), &model.flare_profiles.cop_blue),
            (Kind::SirenBright(true), &model.flare_profiles.cop_bright_red),
            (Kind::SirenBright(false), &model.flare_profiles.cop_bright_blue),
            (Kind::SirenWhite, &model.flare_profiles.cop_white),
            (Kind::SirenOrange, &model.flare_profiles.cop_orange),
        ] {
            for spec in profiles {
                let Some(texture) = model.textures.get(&spec.texture) else { continue };
                let handle = match uploaded.get(&spec.texture) {
                    Some(&handle) => handle,
                    None => {
                        let Some(handle) = blackbox_scene::upload_texture(renderer, texture) else { continue };
                        uploaded.insert(spec.texture, handle);
                        set.textures.push(handle);
                        handle
                    }
                };
                set.profiles.push(Profile { kind, spec: spec.clone(), texture: handle });
            }
        }
        let texture = enhanced::upload(renderer);
        set.textures.push(texture);
        set.enhanced_texture = Some(texture);
        set.mod_lamps = mod_lamps::ModLamps::upload(renderer, model);
        set
    }

    pub fn release(self, renderer: &mut dyn RenderBackend) {
        for texture in self.textures.into_iter().chain(self.mod_lamps.textures()) {
            renderer.destroy_texture(texture);
        }
    }

    pub fn draw(
        &self,
        lamps: &CarLights,
        world: Mat4,
        state: LampState,
        view: FlareView,
        out: &mut Vec<TexturedEffect>,
    ) {
        self.draw_frames(lamps, std::iter::once(world), state, view, false, out);
    }

    /// The PC NOS effect repeats ordinary brake flares at historical car positions.
    pub fn draw_trails(
        &self,
        lamps: &CarLights,
        history: &super::tail_light_trails::TailLightTrails,
        world: Mat4,
        state: LampState,
        view: FlareView,
        out: &mut Vec<TexturedEffect>,
    ) {
        self.draw_frames(lamps, history.anchored(world), state, view, true, out);
    }

    fn draw_frames(
        &self,
        lamps: &CarLights,
        frames: impl Iterator<Item = Mat4> + Clone,
        state: LampState,
        view: FlareView,
        tails_only: bool,
        out: &mut Vec<TexturedEffect>,
    ) {
        let FlareView { camera, forward, fov_degrees, height } = view;
        let Some(focal) = focal_scale(height, fov_degrees) else { return };
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::Y);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        let samples = intensity::samples(lamps, state);
        if !tails_only
            && state.enhanced
            && let Some(texture) = self.enhanced_texture
        {
            enhanced::draw(texture, samples.clone(), frames.clone(), view, focal, out);
        }
        if !tails_only {
            self.mod_lamps.draw(state, frames.clone(), view, out);
        }
        for profile in &self.profiles {
            if tails_only && profile.kind != Kind::Tail {
                continue;
            }
            // A mod car draws the kinds of lamp it gives sprites for itself (`mod_lamps.rs`).
            if self.mod_lamps.covers(profile.kind) {
                continue;
            }
            let mut vertices = Vec::new();
            for world in frames.clone() {
                for (lamp, intensity) in samples.clone() {
                    let kind = match lamp.kind {
                        Kind::CentreBrake => Kind::Tail,
                        other => other,
                    };
                    if kind != profile.kind {
                        continue;
                    }
                    let at = world.transform_point3(lamp.at);
                    let to_eye = camera - at;
                    let depth = (-to_eye).dot(forward);
                    if depth <= 0.3 || !depth.is_finite() {
                        continue;
                    }
                    let facing = match lamp.kind {
                        Kind::Siren(_) | Kind::SirenBright(_) | Kind::SirenWhite | Kind::SirenOrange => 1.0,
                        _ => {
                            world.transform_vector3(lamp.direction).normalize_or_zero().dot(to_eye.normalize_or_zero())
                        }
                    };
                    let Some((radius, color)) =
                        appearance(&profile.spec, intensity, facing, depth / focal, to_eye.length_squared())
                    else {
                        continue;
                    };
                    let at = at + to_eye.normalize_or_zero() * profile.spec.z_bias;
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
                out.push(TexturedEffect { texture: profile.texture, blend: BlendMode::Additive, vertices });
            }
        }
    }
}

pub(super) fn focal_scale(height: u32, fov_degrees: f32) -> Option<f32> {
    if height == 0 || !fov_degrees.is_finite() || !(0.0..180.0).contains(&fov_degrees) {
        return None;
    }
    let focal = height as f32 * 0.5 / (fov_degrees.to_radians() * 0.5).tan();
    (focal.is_finite() && focal > 0.0).then_some(focal)
}

pub(super) fn appearance(
    p: &FlareProfile,
    intensity: f32,
    facing: f32,
    depth_scale: f32,
    distance_squared: f32,
) -> Option<(f32, [u8; 4])> {
    if intensity <= 0.0 || facing <= 0.0 || distance_squared >= 40000.0 {
        return None;
    }
    let intensity = (intensity * facing.powf(p.power)).clamp(0.0, 1.0);
    // The original applies ordered bounds even when the installed minimum exceeds the maximum.
    let scale = depth_scale.max(p.scale[0]).min(p.scale[1]);
    let radius = (p.size[0] + (p.size[1] - p.size[0]) * intensity) * scale;
    let fade = ((40000.0 - distance_squared) / 8000.0).clamp(0.0, 1.0);
    let color = std::array::from_fn(|i| {
        let factor = if i == 3 { 2.0 * fade } else { 1.0 };
        (p.color[i] * intensity * factor * 255.0).clamp(0.0, 255.0) as u8
    });
    (radius > 0.0 && color[3] > 0).then_some((radius, color))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> FlareProfile {
        FlareProfile {
            size: [20.0, 30.0],
            scale: [0.01, 0.04],
            power: 1.0,
            z_bias: 0.3,
            color: [1.0, 0.1, 0.1, 0.2],
            texture: 0,
        }
    }
    #[test]
    fn facing_and_range_reject_hidden_flares() {
        for facing in [-1.0, 0.0] {
            assert!(appearance(&profile(), 1.0, facing, 0.02, 36.0).is_none());
        }
        assert!(appearance(&profile(), 1.0, 1.0, 0.02, 40000.0).is_none());
        let a = appearance(&profile(), 1.0, 1.0, 0.02, 36.0).unwrap();
        let b = appearance(&profile(), 1.0, 0.5, 0.02, 36.0).unwrap();
        assert!(b.0 < a.0 && b.1[3] < a.1[3]);
        assert!(appearance(&profile(), 1.0, 1.0, 0.02, 36000.0).unwrap().1[3] < a.1[3]);
    }
    #[test]
    fn inverted_authored_scale_uses_ordered_bounds() {
        let p = FlareProfile { scale: [1.0, 0.015], ..profile() };
        assert_eq!(appearance(&p, 1.0, 1.0, 0.001, 36.0), appearance(&p, 1.0, 1.0, 0.1, 36.0));
    }
    #[test]
    fn projection_uses_actual_output_size_and_camera_fov() {
        assert!((focal_scale(720, 90.0).unwrap() - 360.0).abs() < 0.001);
        assert!((focal_scale(2160, 90.0).unwrap() - 1080.0).abs() < 0.001);
        assert_eq!(focal_scale(0, 70.0), None);
        for fov in [0.0, -1.0, 180.0, f32::NAN] {
            assert_eq!(focal_scale(720, fov), None);
        }
        let p = FlareProfile { size: [20.0; 2], scale: [0.001, 1.0], ..profile() };
        let low = appearance(&p, 1.0, 1.0, 6.0 / focal_scale(720, 90.0).unwrap(), 36.0).unwrap();
        let high = appearance(&p, 1.0, 1.0, 6.0 / focal_scale(2160, 90.0).unwrap(), 36.0).unwrap();
        assert!((low.0 - high.0 * 3.0).abs() < 0.0001);
    }
}
