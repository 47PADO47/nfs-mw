//! Enhanced NOS ribbons use the actual lamp paths, independently of physics tick count.
use blackbox_gfx::EffectLayer;
use glam::Mat4;

use super::{CarLightFlares, FlareView};
use crate::scenes::world::{
    car_lights::{CarLights, Kind, LampState},
    tail_light_trails::TailLightTrails,
};
use crate::settings::TailTrailStyle;
mod head;
mod shimmer;

impl CarLightFlares {
    #[allow(clippy::too_many_arguments)]
    pub fn draw_smooth_trails(
        &self,
        lamps: &CarLights,
        history: &TailLightTrails,
        world: Mat4,
        state: LampState,
        view: FlareView,
        style: TailTrailStyle,
        bounds: nfsmw_data::car::physics::CarBounds,
        layer: &mut EffectLayer,
    ) {
        if !style.enabled || !style.enhanced {
            return;
        }
        // A car without an installed rear flare profile must not gain invented lamps.
        if !self.profiles.iter().any(|p| p.kind == Kind::Tail) {
            return;
        }
        let out = &mut layer.streaks;
        let first = out.len();
        let response = style.speed_gain(history.speed());
        for (lamp, intensity) in super::intensity::samples(lamps, state) {
            if lamp.kind != Kind::Tail || intensity <= 0.0 || style.width == 0 || style.brightness == 0 {
                continue;
            }
            let at = world.transform_point3(lamp.at);
            let to_eye = view.camera - at;
            let distance2 = to_eye.length_squared();
            if !distance2.is_finite() || distance2 >= 40000.0 || (-to_eye).dot(view.forward) <= 0.3 {
                continue;
            }
            let facing = world.transform_vector3(lamp.direction).normalize_or_zero().dot(to_eye.normalize_or_zero());
            if facing <= 0.0 {
                continue;
            }
            let aperture = lamp.at + lamp.direction.normalize_or_zero() * 0.04;
            let length = (f32::from(style.length) * (1.0 + (response - 1.0) * 0.5)).round().clamp(0.0, 200.0) as u16;
            let mut path = history.path(aperture, length, style.exposure);
            let range_fade = ((40000.0 - distance2) / 8000.0).clamp(0.0, 1.0);
            let gain = intensity
                * facing.powi(2)
                * range_fade
                * history.fade()
                * response
                * history.engagement(style.engagement);
            let brightness = f32::from(style.brightness.min(200)) / 100.0;
            let glow = f32::from(style.glow.min(200)) / 100.0;
            let lens_width = match style.follow_lamp {
                true => (lamps.aperture(lamp.at).y / 0.20).clamp(0.65, 1.6),
                false => 1.0,
            };
            let width = 0.10 * f32::from(style.width.min(200)) / 100.0 * lens_width;
            shimmer::apply(&mut path, history.time(), style.turbulence, width, lamp.at.y * 31.0);
            let softness = f32::from(style.softness.min(200)) / 100.0;
            let halo = 1.0 + 0.65 * softness;
            let core = 0.30 + 0.15 * softness;
            // Emission and coverage are independent. The narrow HDR centre can feed bloom
            // without turning the entire halo into an opaque ribbon.
            for (radius, alpha, emission) in [
                (width * halo, 90.0 * glow * 1.65 / halo, brightness * 1.2),
                (width * core, 210.0 * 0.45 / core, brightness * 3.2),
            ] {
                let tint = style.tint;
                let color = [tint[0], tint[1], tint[2], (alpha * gain).clamp(0.0, 255.0).round() as u8];
                let start = out.len();
                EffectLayer::ribbon(out, &path, radius, color, view.camera, view.forward);
                for vertex in &mut out[start..] {
                    let phase = history.time() * 12.0 - vertex.uv[1] * 6.0 + lamp.at.y * 31.0;
                    let amount = f32::from(style.turbulence.min(200)) / 100.0;
                    vertex.detail[0] = emission * (1.0 + 0.08 * amount * phase.sin());
                }
            }
        }
        let pivot = super::super::space::to_render(bounds.pivot.to_array());
        let half = bounds.half_dimensions;
        let half = glam::Vec3::new(half.z, half.x, half.y);
        EffectLayer::exclude_box(out, first, world.inverse(), pivot - half, pivot + half);
        if style.shape == crate::settings::TrailShape::FlareRibbon {
            head::draw(self, lamps, history, world, state, view, style, bounds, &mut layer.textured);
        }
    }
}
