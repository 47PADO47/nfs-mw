//! The per-frame focus filter handed to the renderer: the speedbreaker vignette and the high-speed radial blur.

use blackbox_gfx::RenderBackend;

use super::WorldScene;

impl WorldScene {
    /// Says once, at init, when the renderer does not draw the high-speed radial blur.
    pub(super) fn note_radial_blur_support(renderer: &dyn RenderBackend) {
        if !renderer.capabilities().radial_blur {
            log::info!("the high-speed radial blur is not drawn by the {} renderer", renderer.capabilities().renderer);
        }
    }

    /// Builds this frame's focus filter and gives it to the renderer.
    pub(super) fn feed_focus(&mut self, renderer: &mut dyn RenderBackend, sim_dt: f32) {
        let mut focus = self.breaker_visuals.update(self.sound_speedbreaker_active, sim_dt);
        let curve = self.traffic.as_ref().map_or(0.0, |traffic| traffic.radial_blur());
        focus.radial_blur = self.tuning.motion_blur.radial_blur(curve);
        renderer.set_focus_filter(focus);
    }
}
