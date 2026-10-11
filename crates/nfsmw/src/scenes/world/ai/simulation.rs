//! Traffic participates in the world's physics tick, including its spawning and pursuit timers.

use blackbox_collision::CollisionWorld;
use blackbox_roads::Body;
use nfsmw_data::car::physics::SurfaceTable;

use super::car::{AiCar, Ctx};
use super::stop::Lights;
use super::{Focus, MANAGER_PERIOD, TrafficWorld};
use crate::scenes::world::{drive::STEP, props::PropWorld};

impl TrafficWorld {
    pub fn breaker_stops(&mut self, areas: Vec<super::super::props::scenery::StopArea>) {
        self.breaker_areas.extend(areas);
    }
    pub(super) fn has_work(&self) -> bool {
        !self.cars.is_empty() || self.target > 0 || self.pursuit.is_some()
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    /// Integrate all AI bodies once. Pair contacts follow before the poses are published.
    pub fn step(&mut self, focus: Focus, collision: &CollisionWorld, props: &mut PropWorld, surfaces: &SurfaceTable) {
        // Every car as the others' trails see it (a hitched trailer is part of its tractor), with whether it is
        // a cop, then the trailers that came loose.
        let mut bodies: Vec<(Body, bool)> = self.cars.iter().map(|c| (c.avoidable(), c.is_cop())).collect();
        bodies.extend(self.cars.iter().filter_map(AiCar::loose_trailer_body).map(|b| (b, false)));
        let lights = self.lights_on.then_some(Lights { controller: &self.signals, time: self.signal_time });
        for (me, car) in self.cars.iter_mut().enumerate() {
            let position = glam::Vec3::from_array(super::super::space::to_physics(car.position()));
            let stop = car.is_cop()
                && self.breaker_areas.iter().any(|area| {
                    area.duration > 0.0 && position.distance_squared(area.position) <= area.radius * area.radius
                });
            car.set_breaker_stop(stop);
            let mut ctx = Ctx {
                net: &self.network,
                index: &self.index,
                rng: &mut self.rng,
                bodies: &bodies,
                me,
                player: focus.body,
                lights,
            };
            car.step(&mut ctx, collision, props, surfaces);
        }
        self.signal_time += STEP;
        for area in &mut self.breaker_areas {
            area.duration -= STEP;
        }
        self.breaker_areas.retain(|a| a.duration > 0.0);
    }

    /// Commit contact poses and advance every gameplay timer by exactly one tick.
    pub fn finish_step(&mut self, focus: Focus, collision: &CollisionWorld, surfaces: &SurfaceTable) {
        for car in &mut self.cars {
            car.sync_contacts();
        }
        let finishing = self.warmup_left > 0.0 && self.warmup_left <= STEP;
        self.warmup_left = (self.warmup_left - STEP).max(0.0);
        self.update_pursuit(STEP, focus, collision, surfaces);
        self.log_timer += STEP;
        if self.log_timer >= 5.0 && log::log_enabled!(log::Level::Debug) {
            self.log_timer = 0.0;
            log::debug!("traffic status: {}", self.status(focus));
        }
        if finishing {
            log::info!("traffic after the screenshot warm-up: {}", self.status(focus));
        }
        self.recycle(STEP, focus);
        self.spawn_timer += STEP;
        if self.spawn_timer >= MANAGER_PERIOD && self.target > 0 {
            self.spawn_timer -= MANAGER_PERIOD;
            self.manage(MANAGER_PERIOD, focus, collision, surfaces);
        }
    }
}
