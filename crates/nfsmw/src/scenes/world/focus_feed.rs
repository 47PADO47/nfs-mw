//! The per-frame focus filter handed to the renderer: the speedbreaker vignette and the high-speed radial blur, and the
//! pursuit look's grade, flash and blur (`pursuit_look.rs`).

use blackbox_gfx::RenderBackend;

use super::WorldScene;

impl WorldScene {
    /// Says once, at init, when the renderer does not draw the high-speed radial blur.
    pub(super) fn note_radial_blur_support(renderer: &dyn RenderBackend) {
        if !renderer.capabilities().radial_blur {
            log::info!("the high-speed radial blur is not drawn by the {} renderer", renderer.capabilities().renderer);
        }
    }

    /// The screen effects of a mod's nitrous variant, added to the filter: its vignette, radial blur and grade follow the
    /// burn with the profile's fades (the camera takes the same level, `camera_choice.rs`). The radial blur is the motion
    /// blur setting's too: off draws none, and the strength scales it. Without a variant the filter is left as it is.
    fn add_nitrous_screen(&mut self, focus: &mut blackbox_gfx::FocusFilter, sim_dt: f32) {
        let screen = self.nitrous_fx.as_deref().and_then(|profile| profile.screen);
        let burning = self.players.lead().is_some_and(|drive| drive.nitrous_burning());
        let level = self.nitrous_screen.step(screen, burning, sim_dt);
        let Some(screen) = screen else { return };
        focus.vignette = focus.vignette.max(screen.vignette * level);
        focus.radial_blur = focus.radial_blur.max(self.tuning.motion_blur.radial_blur(screen.radial_blur * level));
        focus.grade = super::nitrous_screen::grade_for(&screen, level);
    }

    /// Builds this frame's focus filter and gives it to the renderer, with the pursuit look. `sim_dt` is the world's
    /// step, `real_dt` the frame's.
    pub(super) fn feed_focus(&mut self, renderer: &mut dyn RenderBackend, sim_dt: f32, real_dt: f32) {
        let mut focus = self.breaker_visuals.update(self.sound_speedbreaker_active, sim_dt);
        let (speed, burning) = self.players.lead().map_or((0.0, false), |drive| (drive.speed(), drive.nos_burning()));
        let curve = self.breaker_visuals.speed_blur(speed, burning, sim_dt);
        focus.radial_blur = self.tuning.motion_blur.radial_blur(curve);
        self.add_nitrous_screen(&mut focus, sim_dt);
        if let Some(look) = self.pursuit_look_frame(sim_dt, real_dt) {
            focus.radial_blur = focus.radial_blur.max(self.tuning.motion_blur.radial_blur(look.radial_blur));
            renderer.set_colour_look(look.look);
        }
        renderer.set_focus_filter(focus);
    }

    /// A jump's chase started. The original starts it with the jump, inside the reset that never starts the look; the
    /// rewrite starts it once the car stands on its marker, so the look forgets what it knew and the next frame only
    /// records the chase (`docs/specs/pursuit-look.md`).
    pub(in crate::scenes::world) fn jump_chase_started(&mut self) {
        self.pursuit_look.reset();
    }

    /// This frame's pursuit look, from the driven car's heat and whether a chase runs. `None` while the garage shows
    /// its own look or no car is driven; the look then starts again.
    fn pursuit_look_frame(&mut self, sim_dt: f32, real_dt: f32) -> Option<super::pursuit_look::Frame> {
        if self.garage_backdrop.is_some() || self.players.lead().is_none() {
            self.pursuit_look.reset();
            return None;
        }
        if self.nis.arrest_playing() {
            return Some(self.pursuit_look.arrest());
        }
        if self.nis.is_playing() {
            self.pursuit_look.hold();
        }
        let (heat, pursued) = self.traffic.as_ref().map_or((0.0, false), |t| (t.vehicle_heat(), t.is_pursuing()));
        Some(self.pursuit_look.update(heat, pursued, sim_dt, real_dt))
    }
}
