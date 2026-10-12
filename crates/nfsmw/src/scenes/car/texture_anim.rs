//! Animated textures of a model (water, signals): each one shows its current frame, so the surface cycles
//! through the frames the model carries.

use blackbox_gfx::RenderBackend;

use super::CarScene;

impl CarScene {
    /// Show each texture animation's frame for `seconds` after the animations started. The frames are the
    /// textures the model uploaded, so an animation whose frames are missing shows its base texture.
    pub(crate) fn animate_textures(&self, renderer: &mut dyn RenderBackend, seconds: f32) {
        let Some(materials) = &self.materials else { return };
        for anim in &self.texture_anims {
            let base = materials.texture(anim.name_hash);
            let frame = anim.frame_at(seconds).and_then(|hash| materials.texture(hash));
            if let (Some(base), Some(frame)) = (base, frame) {
                renderer.redirect_texture(base, Some(frame));
            }
        }
    }
}
