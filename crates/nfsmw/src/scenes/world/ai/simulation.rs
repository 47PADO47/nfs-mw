//! Traffic participates in the world's physics tick, including its spawning and pursuit timers.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::breaker;
use blackbox_roads::Body;
use glam::Vec2;
use nfsmw_data::car::physics::SurfaceTable;

use super::car::{AiCar, Ctx};
use super::stop::Lights;
use super::{Focus, MANAGER_PERIOD, TrafficWorld};
use crate::scenes::world::{drive::STEP, props::PropWorld};
use crate::viewer::camera::Lens;

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
        let aims = focus.body.as_ref().map(|player| self.cop_aims(player)).unwrap_or_default();
        for (me, car) in self.cars.iter_mut().enumerate() {
            let position = glam::Vec3::from_array(super::super::space::to_physics(car.position()));
            let point = Vec2::new(position.x, position.z);
            let stop = car.is_roadblock()
                || (car.is_cop()
                    && self.breaker_areas.iter().any(|area| {
                        area.duration > 0.0
                            && breaker::inside(Vec2::new(area.position.x, area.position.z), area.radius, point)
                    }));
            car.set_breaker_stop(stop);
            car.set_avoidance(self.avoidance);
            car.set_player_speed(focus.body.map_or(0.0, |b| b.velocity.length()));
            let mut ctx = Ctx {
                net: &self.network,
                index: &self.index,
                rng: &mut self.rng,
                bodies: &bodies,
                me,
                player: focus.body,
                aims: &aims,
                lights,
            };
            car.step(&mut ctx, collision, props, surfaces);
            if car.take_wreck() {
                self.police.record_wreck();
            }
            car.step_lights();
        }
        self.signal_time += STEP;
        for area in &mut self.breaker_areas {
            area.duration -= STEP;
        }
        self.breaker_areas.retain(|a| a.duration > 0.0);
    }

    /// Commit contact poses and advance every gameplay timer by exactly one tick. `camera` is the lens the
    /// player sees the traffic through (the simulation camera), which decides what counts as seen; `props` holds
    /// the detached parts of the cars.
    pub fn finish_step(
        &mut self,
        focus: Focus,
        camera: &Lens,
        props: &PropWorld,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) {
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
        self.recycle(STEP, focus, camera, props);
        self.spawn_timer += STEP;
        if self.spawn_timer >= MANAGER_PERIOD && self.target > 0 {
            self.spawn_timer -= MANAGER_PERIOD;
            self.manage(MANAGER_PERIOD, focus, collision, surfaces);
        }
    }
}
