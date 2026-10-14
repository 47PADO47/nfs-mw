//! A single 60 Hz timeline for the player, traffic, object contacts and world timers.

use blackbox_gfx::RenderBackend;

use super::{View, WorldScene, ai, drive::STEP};
use crate::input::ActionState;

/// A screenshot update has no elapsed frame time, so it runs this many shared ticks.
const BATCH_STEPS: u32 = 6;

impl WorldScene {
    pub(super) fn update_simulation(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) -> f32 {
        self.prepare_traffic(renderer);
        let speedbreaker = self.tuning.speedbreaker;
        self.speedbreaker_clock.set_dilation(speedbreaker.dilation);
        if let Some(drive) = self.drive.as_mut() {
            drive.set_speedbreaker_tuning(speedbreaker);
        }
        let driving = self.view == View::Chase;
        if let Some(drive) = self.drive.as_mut()
            && driving
        {
            drive.prepare_frame(input, dt);
        }
        let batch = dt == 0.0
            && ((driving && self.drive.as_ref().is_some_and(|d| d.needs_batch()))
                || self.traffic.as_ref().is_some_and(|t| !t.settled()));
        // Only missing local world data pauses simulation, not loading distant visible scenery.
        let speed = self.drive.as_ref().map_or(0.0, |drive| drive.speed().abs());
        let loaded = self.residency.simulation_ready(self.focus(), 20.0 + speed * dt.max(STEP));
        let real_dt = match batch {
            true => BATCH_STEPS as f32 * STEP,
            false => dt,
        };
        let sim_dt = match loaded {
            false => 0.0,
            true => {
                let active = self.drive.as_mut().filter(|_| driving).is_some_and(|drive| drive.prepare_time());
                self.speedbreaker_clock.advance(active, real_dt) * self.tuning.game_speed
            }
        };
        if loaded && let Some(drive) = self.drive.as_mut().filter(|_| driving) {
            drive.finish_time(input, sim_dt, real_dt);
        }
        if let Some(drive) = self.drive.as_mut()
            && (!driving || drive.waiting_for_road().is_some())
        {
            drive.age_effects(sim_dt);
        }
        let steps = self.physics_clock.advance(sim_dt);
        let look = self.drive.as_mut().map_or((0.0, 0.0), |drive| drive.take_look(steps));
        for _ in 0..steps {
            self.physics_step(driving, look);
        }
        // The speeding trigger reads the sighting the collision step just made, checked against the walls.
        if self.drive.is_some() {
            let (collision, _) = self.residency.world_parts();
            if let Some(traffic) = self.traffic.as_mut() {
                let cast = super::drive::sight::sight_cast(collision);
                traffic.update_sight(steps as f32 * STEP, &cast);
            }
            ai::speeding_pursuit(self, renderer);
            // The helicopter's sight asks the world whether the line to the player is clear.
            let (collision, _) = self.residency.world_parts();
            if let (Some(drive), Some(traffic)) = (self.drive.as_ref(), self.traffic.as_mut()) {
                let view = drive.player_view();
                let cast = super::drive::walls::world_cast(collision);
                let clear = |from: glam::Vec3, to: glam::Vec3| cast(from, to).is_none();
                // The simulated time of this update, as the traffic gets it: a screenshot run has no frame time, so
                // a per-frame dt would stop the police clock offscreen.
                traffic.update_police(steps as f32 * STEP, &view, &clear);
                // A support roll asked for a roadblock: the world places it, and the chase keeps no second one.
                if traffic.take_roadblock_request() {
                    match traffic.place_roadblock(view, collision, &self.physics.surfaces) {
                        Ok(cars) => {
                            log::info!("support: a roadblock of {cars} cop cars");
                            traffic.roadblock_placed();
                        }
                        Err(e) => log::warn!("support: no roadblock: {e}"),
                    }
                }
            }
        }
        // A loading pause holds the last displayed phase instead of jumping ahead and rewinding.
        let alpha = match batch || dt == 0.0 {
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
            drive.trace_frame(dt, steps, loaded, self.residency.complete());
        }
        sim_dt
    }

    /// Every moving car integrates before pair contacts, and every pose is captured after those contacts.
    fn physics_step(&mut self, driving: bool, look: (f32, f32)) {
        let focus = self.traffic_focus();
        let event_view = self.simulation_lens(self.physics_aspect).position;
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
            traffic.update_events(props, &self.physics, event_view);
            traffic.finish_step(focus, collision, surfaces);
        }
        if let (Some(drive), Some(step)) = (self.drive.as_mut(), player_step) {
            drive.update_events(props, &self.physics, event_view);
            drive.finish_step(collision, surfaces, step);
        }
        let stops = props.update_scenery(&self.physics);
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.breaker_stops(stops);
        }
        if let Some(drive) = self.drive.as_mut().filter(|_| driving) {
            drive.follow(collision, look, STEP);
        }
        let view = self.simulation_lens(self.physics_aspect).view_proj();
        let (collision, props) = self.residency.world_parts();
        props.update_lifetimes(&super::drive::WorldGround { collision, surfaces }, view, STEP);
    }
}
