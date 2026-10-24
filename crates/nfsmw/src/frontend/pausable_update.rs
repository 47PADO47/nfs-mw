//! Updates and deferred effects for the resident world beneath front-end menus.
use super::*;

impl Pausable {
    pub(super) fn advance_scene(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        // A multiplayer client's world never stops (the server does not pause): it runs on under the menu with
        // neutral input.
        if self.paused.get() && self.inner.runs_while_paused() {
            self.inner.update(renderer, &ActionState::default(), dt);
            return;
        }
        if self.paused.get() {
            if self.effects_dirty {
                self.inner.refresh_effects(renderer);
                self.effects_dirty = false;
            }
            return;
        }
        self.inner.update(renderer, input, dt);
        self.effects_dirty = false;
    }
}
