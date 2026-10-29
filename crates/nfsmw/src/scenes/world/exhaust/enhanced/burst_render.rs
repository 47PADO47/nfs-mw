//! Compact transient volumes using the existing licensed fire lookup and noise.
use blackbox_gfx::{FlameAppearance, FlameVolume, TextureHandle};
use glam::{Mat4, Vec3};

use super::{Enhanced, motion::Motion};

impl Enhanced {
    /// The factor on the size of a burst: a bike's flick is smaller than a car's (`trigger::BIKE_FLAME_SCALE`).
    fn flame_scale(&self) -> f32 {
        match self.bike {
            true => super::super::trigger::BIKE_FLAME_SCALE,
            false => 1.0,
        }
    }

    pub(super) fn burst_geometry(
        &self,
        pipes: &[Mat4],
        texture: TextureHandle,
        motion: &Motion,
        out: &mut Vec<FlameVolume>,
    ) -> bool {
        let style = &self.backfire_style;
        if style.size == 0 || style.brightness == 0 || style.duration == 0 {
            return true;
        }
        let preview = self.burst_preview.is_some();
        let (bursts, elapsed) = match self.burst_preview.as_ref() {
            Some((start, bursts)) => (bursts, start.elapsed().as_secs_f32()),
            None => (self.visual_bursts.as_ref().unwrap_or(&self.bursts), 0.0),
        };
        for flash in bursts.frames(style, elapsed) {
            if flash.strength < 0.01 {
                continue;
            }
            for (index, pipe) in pipes.iter().enumerate() {
                let seed = flash.seed + index as f32 * 7.17;
                let variation = f32::from(style.variation) / 100.0;
                let size = (1.0 + seed.sin() * 0.07 * variation) * self.flame_scale();
                let length = flash.length * size;
                let width = flash.width * size;
                let marker = self.pose * *pipe;
                let jet = marker.transform_vector3(Vec3::Z).normalize_or_zero();
                let velocity = match preview {
                    true => Vec3::ZERO,
                    false => motion.velocity,
                };
                let crosswind = -velocity + jet * velocity.dot(jet);
                // Short-lived gas reacts to airflow but has little time to rise.
                let wind = crosswind * flash.age * 0.06;
                let angular = match preview {
                    true => Vec3::ZERO,
                    false => motion.angular_velocity,
                };
                let turn = -angular.cross(jet) * flash.age * length * 0.5;
                let force = wind + turn + Vec3::Z * length * flash.phase * 0.035;
                let bend = (force - jet * force.dot(jet)).clamp_length_max(length * 0.4);
                let proxy = width * 1.5 + 2.0 * bend.length();
                let to_world = marker
                    * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)
                    * Mat4::from_scale(Vec3::new(proxy, length, proxy))
                    * Mat4::from_translation(Vec3::Y * 0.5);
                let colour = |rgb: [u8; 3]| Vec3::from_array(rgb.map(|v| f32::from(v) / 255.0)).extend(1.0).to_array();
                let appearance = FlameAppearance {
                    root_color: colour(style.root),
                    tail_color: colour(style.tail),
                    density: 1.25 * (1.0 - 0.45 * flash.phase),
                    turbulence: flash.turbulence,
                    hot_core: 1.0,
                    radial_scale: proxy / width,
                    bend: to_world.inverse().transform_vector3(bend),
                    ..Default::default()
                };
                out.push(FlameVolume {
                    texture,
                    to_world,
                    time: flash.age * 9.0 + flash.seed,
                    seed,
                    blue: false,
                    intensity: flash.strength * 0.9 * f32::from(style.brightness) / 100.0,
                    appearance,
                });
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bikes_bursts_are_drawn_smaller_than_a_cars() {
        let mut enhanced = Enhanced::default();
        assert_eq!(enhanced.flame_scale(), 1.0);
        enhanced.bike = true;
        assert_eq!(enhanced.flame_scale(), super::super::super::trigger::BIKE_FLAME_SCALE);
        assert!(enhanced.flame_scale() < 1.0);
    }
}
