//! The settings as the renderer takes them: one [`GraphicsSettings`] request (docs/renderers.md).
//!
//! This is only the *request*. What a renderer actually runs is decided by `blackbox_gfx::resolve` against its
//! capabilities, after the graphics preset has filled in the keys the player did not set.

use blackbox_gfx::{AoSettings, GraphicsSettings};

use super::{Settings, post_effects};

impl Settings {
    /// What the settings ask of the renderer: the post effects, the upscaler with its render scale and sharpness.
    pub fn graphics(&self) -> GraphicsSettings {
        GraphicsSettings {
            reflections: self.reflections,
            reflection_resolution: blackbox_gfx::reflection::ReflectionResolution {
                cube: self.reflection_cube_size,
                mirror: self.reflection_mirror_size,
                pow2: self.reflection_pow2,
            },
            mirror_tint: self.mirror_tint,
            shadows: self.shadows,
            post: post_effects(self),
            upscaler: self.upscaler.upscaler(),
            render_scale: self.render_scale.factor(),
            upscale_sharpness: self.upscale_sharpness.amplitude(),
            upscale_quality: self.upscale_quality.mode(),
            ray_tracing: self.ray_tracing.level(),
            ao: AoSettings {
                quality: self.ao,
                intensity: self.ao_intensity.amplitude(),
                compare: self.ao_compare,
                ..AoSettings::default()
            },
        }
    }
}
