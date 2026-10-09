//! The settings as the renderer takes them: one [`GraphicsSettings`] request (docs/renderers.md).
//!
//! This is only the *request*. What a renderer actually runs is decided by `blackbox_gfx::resolve` against its
//! capabilities, after the graphics preset has filled in the keys the player did not set.

use blackbox_gfx::GraphicsSettings;

use super::{Settings, post_effects};

impl Settings {
    /// What the settings ask of the renderer: the post effects, the upscaler with its render scale and sharpness.
    pub fn graphics(&self) -> GraphicsSettings {
        GraphicsSettings {
            post: post_effects(self),
            upscaler: self.upscaler.upscaler(),
            render_scale: self.render_scale.factor(),
            upscale_sharpness: self.upscale_sharpness.amplitude(),
            ..GraphicsSettings::default()
        }
    }
}
