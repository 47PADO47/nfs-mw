//! Runtime-installed PC precipitation textures; no generated replacement sprites.
use super::super::WorldScene;
use blackbox_gfx::{BlendMode, DistortionRegion, EffectLayer, ScreenDistortion, TexturedEffect};
use blackbox_hash::bstring_hash;
use glam::Vec3;

pub(super) struct RainEffects {
    falling: Option<TexturedEffect>,
    lens: Option<ScreenDistortion>,
}
impl RainEffects {
    pub(super) fn append(self, layer: &mut EffectLayer) {
        if let Some(batch) = self.falling {
            layer.textured.push(batch);
        }
        layer.distortion = self.lens;
    }
}
impl WorldScene {
    pub(super) fn rain_effects(&self, camera: Vec3) -> RainEffects {
        let textures = &self.residency.shared.materials;
        let falling = textures
            .get(&bstring_hash("RAINDROP"))
            .map(|&(texture, _)| {
                let mut vertices = Vec::with_capacity(self.rain.falling() * 6);
                let mut quads = Vec::with_capacity(self.rain.falling());
                self.rain.quads(camera, &mut quads);
                for (quad, cell) in quads {
                    let u = cell as f32 * 0.1;
                    EffectLayer::particle_quad_uv(&mut vertices, quad, [128; 4], [0.0; 2], [[u, 0.0], [u + 0.1, 1.0]]);
                }
                TexturedEffect { texture, blend: BlendMode::AlphaBlend, vertices }
            })
            .filter(|effect| !effect.vertices.is_empty());
        let lens = textures
            .get(&bstring_hash("RAINDROPPC"))
            .map(|&(texture, _)| {
                let regions = self
                    .rain
                    .lens()
                    .iter()
                    .filter(|d| d.remaining > 0.0 && d.size > 0.0 && self.tuning.rain_droplet_scale > 0.0)
                    .map(|d| lens_region(d, self.tuning.rain_droplet_scale))
                    .collect();
                ScreenDistortion { texture, regions }
            })
            .filter(|effect| !effect.regions.is_empty());
        RainEffects { falling, lens }
    }
}

fn lens_region(d: &crate::weather::lens::Droplet, scale: f32) -> DistortionRegion {
    let size = d.size * 0.12;
    let [x, y] = d.position.to_array();
    DistortionRegion {
        rect: [
            x + size * 0.25 * (1.0 - scale),
            y - size * 0.25 * (1.0 + scale),
            size * 0.5 * scale,
            size * 0.5 * scale,
        ],
        uv_rect: [0.0, 1.0, 1.0, -1.0],
        sample_transform: [
            x + size * (0.25 - 0.5 * scale),
            y + size * (0.5 * scale - 0.25),
            size * scale,
            -size * scale,
        ],
        opacity: 3.0 * d.remaining / d.lifetime,
        brightness: 1.2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lens_size_preserves_position_and_refraction_ratio() {
        let mut rain = crate::weather::lens::LensRain::default();
        rain.update(0.016, 0.0, true);
        let d = &rain.visible(true)[0];
        let original = lens_region(d, 1.0);
        let centre = |r: [f32; 4]| [r[0] + r[2] * 0.5, r[1] + r[3] * 0.5];
        for scale in [0.0, 0.25, 0.5, 1.0, 2.0] {
            let region = lens_region(d, scale);
            for (a, b) in centre(region.rect).into_iter().zip(centre(original.rect)) {
                assert!((a - b).abs() < 1e-6);
            }
            for (a, b) in centre(region.sample_transform).into_iter().zip(centre(original.sample_transform)) {
                assert!((a - b).abs() < 1e-6);
            }
            assert!((region.rect[2] - original.rect[2] * scale).abs() < 1e-6);
            assert!((region.sample_transform[2] - 2.0 * region.rect[2]).abs() < 1e-6);
            assert_eq!(region.opacity, original.opacity);
        }
    }
}
