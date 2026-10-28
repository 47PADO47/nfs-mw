//! Short textured flare impression feeding the continuous tail, using installed profiles.
use super::super::{CarLightFlares, FlareView, appearance, focal_scale, intensity};
use crate::scenes::world::{
    car_lights::{CarLights, Kind, LampState},
    tail_light_trails::TailLightTrails,
};
use crate::settings::TailTrailStyle;
use blackbox_gfx::{BlendMode, EffectLayer, TexturedEffect};
use glam::{Mat4, Vec3};
use nfsmw_data::car::physics::CarBounds;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    flares: &CarLightFlares,
    lamps: &CarLights,
    history: &TailLightTrails,
    world: Mat4,
    state: LampState,
    view: FlareView,
    style: TailTrailStyle,
    bounds: CarBounds,
    out: &mut Vec<TexturedEffect>,
) {
    if style.head_size == 0 || style.lamp_bloom == 0 {
        return;
    }
    let Some(focal) = focal_scale(view.height, view.fov_degrees) else { return };
    let right = view.forward.cross(Vec3::Z).normalize_or(Vec3::Y);
    let up = right.cross(view.forward).normalize_or(Vec3::Z);
    let size = f32::from(style.head_size.min(200)) / 100.0;
    for profile in flares.profiles.iter().filter(|p| p.kind == Kind::Tail).take(2) {
        let mut vertices = Vec::new();
        for (lamp, light) in intensity::samples(lamps, state) {
            if lamp.kind != Kind::Tail || light <= 0.0 {
                continue;
            }
            let path = history.path(lamp.at + lamp.direction.normalize_or_zero() * 0.04, style.length, style.exposure);
            let Some(&head) = path.first() else { continue };
            let to_eye = view.camera - head;
            let depth = (-to_eye).dot(view.forward);
            if depth <= 0.3 {
                continue;
            }
            let facing = world.transform_vector3(lamp.direction).normalize_or_zero().dot(to_eye.normalize_or_zero());
            let Some((radius, mut color)) =
                appearance(&profile.spec, light, facing, depth / focal, to_eye.length_squared())
            else {
                continue;
            };
            let radius = radius.clamp(0.10, 0.22) * size;
            color[..3].copy_from_slice(&style.tint);
            let base_alpha = f32::from(color[3]) * history.fade() * history.engagement(style.engagement) * 0.55;
            let mut travelled = 0.0;
            let mut next = 0.0;
            let mut count = 0;
            for i in 0..path.len() {
                if i > 0 {
                    travelled += path[i].distance(path[i - 1]);
                }
                if travelled < next {
                    continue;
                }
                if count == 3 || travelled > 0.55 {
                    break;
                }
                let at = path[i] + (view.camera - path[i]).normalize_or_zero() * 0.025;
                let dimensions = match style.follow_lamp {
                    true => lamps.aperture(lamp.at),
                    false => Vec3::new(0.0, 0.20, 0.11),
                };
                let x = right * radius * (dimensions.y / 0.20).clamp(0.65, 1.8);
                let y = up * radius * (dimensions.z / 0.20).clamp(0.3, 0.9);
                color[3] = (base_alpha * (1.0 - travelled / 0.6).max(0.0)).round() as u8;
                let emission = f32::from(style.lamp_bloom.min(200)) / 100.0 * style.speed_gain(history.speed());
                EffectLayer::particle_quad(
                    &mut vertices,
                    [at - x + y, at + x + y, at + x - y, at - x - y],
                    color,
                    [emission, 0.0],
                );
                next += 0.16;
                count += 1;
            }
        }
        let pivot = super::super::super::space::to_render(bounds.pivot.to_array());
        let half = Vec3::new(bounds.half_dimensions.z, bounds.half_dimensions.x, bounds.half_dimensions.y);
        EffectLayer::exclude_box(&mut vertices, 0, world.inverse(), pivot - half, pivot + half);
        if !vertices.is_empty() {
            out.push(TexturedEffect { texture: profile.texture, blend: BlendMode::Additive, vertices });
        }
    }
}
