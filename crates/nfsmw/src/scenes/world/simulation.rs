//! A single 60 Hz timeline for the player, traffic, object contacts and world timers.

use blackbox_render::Renderer;

use super::{View, WorldScene, drive::STEP};
use crate::input::{Action, ActionState};

/// A screenshot update has no elapsed frame time, so it runs this many shared ticks.
const BATCH_STEPS: u32 = 6;

impl WorldScene {
    pub(super) fn update_simulation(&mut self, renderer: &mut Renderer, input: &ActionState, dt: f32) {
        self.prepare_traffic(renderer);
        let driving = self.view == View::Chase;
        if let Some(drive) = self.drive.as_mut() {
            if driving {
                drive.prepare_frame(input, dt);
            }
            if !driving || drive.waiting_for_road().is_some() {
                drive.age_effects(dt);
            }
        }
        let batch = dt == 0.0
            && ((driving && self.drive.as_ref().is_some_and(|d| d.needs_batch()))
                || self.traffic.as_ref().is_some_and(|t| !t.settled()));
        // Streaming pauses the shared world; one car must not advance through frozen traffic.
        let loaded = self.residency.complete();
        let steps = match (loaded, batch) {
            (false, _) => {
                self.physics_clock.reset();
                0
            }
            (true, true) => BATCH_STEPS,
            (true, false) => self.physics_clock.advance(dt),
        };
        for _ in 0..steps {
            self.physics_step(driving, (input.value(Action::LookX), input.value(Action::LookY)));
        }
        let alpha = match batch || !loaded || dt == 0.0 {
            true => 1.0,
            false => self.physics_clock.alpha(),
        };
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.set_alpha(alpha);
        }
        self.residency.world_parts().1.set_alpha(alpha);
        if let Some(drive) = self.drive.as_mut() {
            drive.set_alpha(match driving {
                true => alpha,
                false => 1.0,
            });
        }
    }

    /// Every moving car integrates before pair contacts, and every pose is captured after those contacts.
    fn physics_step(&mut self, driving: bool, look: (f32, f32)) {
        let focus = self.traffic_focus();
        let (collision, props) = self.residency.world_parts();
        let surfaces = &self.physics.surfaces;
        props.step(&super::drive::WorldGround { collision, surfaces }, STEP);
        let player_step = match (driving, self.drive.as_mut()) {
            (true, Some(drive)) => drive.step(collision, props, surfaces),
            _ => None,
        };
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.step(focus, collision, props, surfaces);
            let player = self.drive.as_mut().filter(|_| player_step.is_some()).and_then(|d| d.sim_mut());
            traffic.collide(player);
            traffic.update_events(props, &self.physics, self.camera.position);
            traffic.finish_step(focus, collision, surfaces);
        }
        if let (Some(drive), Some(step)) = (self.drive.as_mut(), player_step) {
            drive.update_events(props, &self.physics, self.camera.position);
            drive.finish_step(collision, surfaces, step);
        }
        let stops = props.update_scenery(&self.physics);
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.breaker_stops(stops);
        }
        if let Some(drive) = self.drive.as_mut().filter(|_| driving) {
            drive.follow(collision, look, STEP);
        }
        let view = match self.drive.as_ref().filter(|_| self.view == View::Chase) {
            Some(drive) => drive.camera().view_proj(self.physics_aspect),
            None => self.camera.view_proj(self.physics_aspect),
        };
        props.update_lifetimes(&super::drive::WorldGround { collision, surfaces }, view, STEP);
    }
}
