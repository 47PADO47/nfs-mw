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

    /// The screen effects of a mod's nitrous variant, added to the filter: its vignette and radial blur follow the burn
    /// with the profile's fades. Without a variant the filter is left as it is.
    fn add_nitrous_screen(&mut self, focus: &mut blackbox_gfx::FocusFilter, sim_dt: f32) {
        let Some(screen) = self.nitrous_fx.as_deref().and_then(|profile| profile.screen) else {
            self.nitrous_burn = Default::default();
            return;
        };
        let burning = self.players.lead().is_some_and(|drive| drive.nitrous_burning());
        let step = self.nitrous_burn.step(burning, screen.fade_in, screen.fade_out, sim_dt);
        focus.vignette = focus.vignette.max(screen.vignette * step.level);
        focus.radial_blur = focus.radial_blur.max(screen.radial_blur * step.level);
    }

    /// Builds this frame's focus filter and gives it to the renderer.
    pub(super) fn feed_focus(&mut self, renderer: &mut dyn RenderBackend, sim_dt: f32) {
        let mut focus = self.breaker_visuals.update(self.sound_speedbreaker_active, sim_dt);
        let curve = self.traffic.as_ref().map_or(0.0, |traffic| traffic.radial_blur());
        focus.radial_blur = self.tuning.motion_blur.radial_blur(curve);
        self.add_nitrous_screen(&mut focus, sim_dt);
        renderer.set_focus_filter(focus);
    }
}
