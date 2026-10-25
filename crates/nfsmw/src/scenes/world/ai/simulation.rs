//! Traffic participates in the world's physics tick, including its spawning and pursuit timers.

use blackbox_collision::CollisionWorld;
use glam::Vec2;
use nfsmw_data::car::physics::SurfaceTable;

use super::car::{AiCar, Ctx};
use super::stop::Lights;
use super::{Focus, TrafficWorld, activation, parallel, police::Police};
#[cfg(test)]
use crate::scenes::world::drive::STEP;
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::props::PropWorld;
use crate::viewer::camera::Lens;

impl TrafficWorld {
    pub fn set_damage_view(&mut self, view: glam::Vec3) {
        self.damage_view = Some(view);
    }
    /// Takes the zones the scenery events opened this tick (`EBreakerStopCops`).
    pub fn breaker_stops(&mut self, areas: Vec<super::super::props::scenery::StopArea>) {
        for area in areas {
            if area.gas_station {
                // The breaker's own sequence blew it up: the player's chase set it off.
                let at = area.position.to_array();
                self.note_speech(crate::audio::WorldEvent::GasStation { by_player: true, at });
            }
            self.breaker_zones.push(Vec2::new(area.position.x, area.position.z), area.radius, area.duration);
        }
    }

    /// The live pursuit breaker zones as (centre x, centre z, radius, seconds left), physics space.
    pub fn breaker_zone_list(&self) -> Vec<(f32, f32, f32, f32)> {
        self.breaker_zones.iter().map(|z| (z.centre.x, z.centre.y, z.radius, z.remaining)).collect()
    }

    pub(super) fn has_work(&self) -> bool {
        !self.cars.is_empty() || self.target > 0 || self.pursuit.is_some() || self.patrol_enabled()
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
        self.police.set_heli_alpha(alpha);
    }

    /// Integrate all AI bodies once. Pair contacts follow before the poses are published.
    ///
    /// With enough cars the integrations run on several threads (`parallel.rs`): every car's start first, in car order,
    /// then the integrations, then every car's end in car order. The cars come out bit for bit as the plain order
    /// leaves them, each car's whole step before the next car's.
    #[cfg(test)]
    pub fn step(&mut self, focus: Focus, collision: &CollisionWorld, props: &mut PropWorld, surfaces: &SurfaceTable) {
        self.step_with_delta(STEP, focus, collision, props, surfaces);
    }

    /// One ordered shared tick with its simulation delta, independent of render cadence.
    pub fn step_with_delta(
        &mut self,
        dt: f32,
        focus: Focus,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.step_delta = dt;
        // Every car as the others' trails see it (a hitched trailer is part of its tractor), with whether it is
        // a cop, then the trailers that came loose.
        let mut bodies = std::mem::take(&mut self.bodies);
        bodies.clear();
        bodies.extend(self.cars.iter().map(|c| (c.avoidable(), c.is_cop())));
        bodies.extend(self.cars.iter().filter_map(AiCar::loose_trailer_body).map(|b| (b, false)));
        bodies.extend(self.guest_bodies().map(|b| (b, false)));
        self.watch_ground(focus.body.map(|b| b.position), collision);
        let target = self.pursuit_target(focus.body);
        // Normal driving establishes the vehicle's road history before any chase.
        // A running pursuit may use only its published visible or remembered target.
        let tracking_body = if self.pursuit.is_none() { focus.body } else { target };
        let racing = self.police.pursuit_context().racing;
        let target_road = self.target_road.update(
            &self.network,
            &self.index,
            tracking_body,
            racing,
            self.police.is_jerk(),
            self.step_delta,
        );
        let lights = self.lights_on.then_some(Lights { controller: &self.signals, time: self.signal_time });
        let mut aims = std::mem::take(&mut self.aims);
        aims.clear();
        if let Some(player) = target.as_ref() {
            self.cop_aims(player, collision, &mut aims);
        }
        let chasers = self.secondary_assignments(target);
        for (aim, chaser) in aims.iter_mut().zip(&chasers) {
            // A cop sent after a secondary target drives at it (`None` is the car it chases).
            if chaser.is_some() {
                *aim = None;
            }
        }
        self.population_stats.waiting_collision = 0;
        let threads = self.parallel.threads(self.cars.len());
        let mut starts = std::mem::take(&mut self.starts);
        starts.clear();
        let searching = !self.police.is_perp_in_sight();
        let search_cruise = self.search_cruise();
        // A car's own crash damage is gated by the nearest player's view (`collide.rs`).
        let eyes: Vec<glam::Vec3> = self.guest_eyes().collect();
        for (me, car) in self.cars.iter_mut().enumerate() {
            car.set_step_delta(dt);
            let secondary = chasers.get(me).copied().flatten();
            let shared_target_road = !racing && secondary.is_none();
            let view = self.damage_view.or_else(|| focus.body.map(|b| b.position));
            car.damage_distance = super::population::view_distance(view, &eyes, car.physics_position());
            car.search_mode(searching, search_cruise, &self.network, &self.index);
            let position = glam::Vec3::from_array(super::super::space::to_physics(car.position()));
            let point = Vec2::new(position.x, position.z);
            // A police car inside a breaker zone is destroyed, however it got there (`ApplyBreakerZones`); it is counted
            // as a wreck below, like one from damage. A roadblock car holds its place with the brakes on.
            if car.is_live_police() && self.breaker_zones.covers(point) {
                log::info!("pursuit breaker: {} destroyed at {:?}", car.name, point);
                car.destroy_by_breaker();
            }
            car.set_breaker_stop(car.is_roadblock());
            car.set_avoidance(self.avoidance);
            car.set_player_speed(focus.body.map_or(0.0, |b| b.velocity.length()));
            let head_on_support = car.head_on_support();
            if let super::car::Role::Cop(cop) = &mut car.role {
                cop.environment(super::cop::Environment {
                    reaction: self.police.level_tuning(self.police.level().unwrap_or(1)).map(|row| row.speed_reaction),
                    level: self.police.level().unwrap_or(1),
                    nitrous: self.police.target_nitrous(),
                    nitrous_input: self.police.target_nitrous_input(),
                    nitrous_boost: self.police.target_nitrous_boost(),
                    jerk: self.police.is_jerk(),
                    racing,
                    head_on_support,
                    shared_target_road,
                    target_road: if shared_target_road { target_road } else { None },
                });
            }
            let mut ctx = Ctx {
                net: &self.network,
                index: &self.index,
                rng: &mut self.rng.nav,
                bodies: &bodies,
                me,
                player: match car.is_cop() && !car.is_searching() {
                    true => secondary.or(target),
                    false => focus.body,
                },
                aims: &aims,
                lights,
            };
            let ready = collision.resident_near(car.physics_position().to_array(), 30.0) != Some(false);
            car.set_sight_active(ready);
            // A racer off the loaded world does not wait for it: its stand-in drives on (`car/stand_in.rs`).
            if car.update_stand_in(ready, &WorldGround { collision, surfaces }) {
                car.stand_in_step(&mut ctx, collision);
                if threads > 1 {
                    starts.push(None);
                    continue;
                }
                car_after_step(car, &mut self.police);
                continue;
            }
            self.population_stats.waiting_collision += usize::from(!ready);
            if threads > 1 {
                starts.push(match ready {
                    true => car.begin_step(&mut ctx, props, collision),
                    false => None,
                });
                continue;
            }
            if ready {
                car.step(&mut ctx, collision, props, surfaces);
            }
            car_after_step(car, &mut self.police);
        }
        if threads > 1 {
            parallel::integrate(&mut self.cars, &starts, collision, surfaces, threads);
            for (car, start) in self.cars.iter_mut().zip(&starts) {
                if let Some(start) = start {
                    car.finish_step(start, collision, props, surfaces);
                }
                car_after_step(car, &mut self.police);
            }
        }
        self.watch_cops_against_walls();
        self.bodies = bodies;
        self.aims = aims;
        self.starts = starts;
        self.signal_time += self.step_delta;
        self.breaker_zones.step(self.step_delta);
    }

    /// Commit contact poses and advance every gameplay timer by exactly one tick. `camera` is the lens the
    /// player sees the traffic through (the simulation camera), which decides what counts as seen; `props` holds
    /// the detached parts of the cars.
    pub fn finish_step(
        &mut self,
        focus: Focus,
        camera: &Lens,
        shown: &Lens,
        props: &PropWorld,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) {
        self.cops.set_eye(glam::Vec3::from(crate::scenes::world::space::to_physics(camera.position)));
        for car in &mut self.cars {
            car.sync_contacts();
            if car.is_damaged_cop()
                && !car.is_patrol()
                && let Some(pursuit) = &mut self.pursuit
            {
                pursuit.damaged.insert(car.audio_emitter().id);
            }
        }
        let finishing = self.warmup_left > 0.0 && self.warmup_left <= self.step_delta;
        self.warmup_left = (self.warmup_left - self.step_delta).max(0.0);
        self.update_pursuit(self.step_delta, focus, collision, surfaces);
        self.apply_breakers_to_helicopter();
        self.log_timer += self.step_delta;
        if self.log_timer >= 5.0 && log::log_enabled!(log::Level::Debug) {
            self.log_timer = 0.0;
            log::debug!("traffic status: {}", self.status(focus));
        }
        if finishing {
            log::info!("traffic after the screenshot warm-up: {}", self.status(focus));
        }
        self.recycle(self.step_delta, focus, camera, shown, props);
        self.service_spawn_requests(focus, collision, surfaces);
        let enabled = self.target > 0 || self.patrol_enabled();
        if activation::due(&mut self.spawn_timer, self.step_delta, enabled) {
            let at = self.next_spawn_focus(focus);
            self.manage(activation::PERIOD, at, collision, surfaces);
        }
    }
}

/// What follows a car's step in car order, stepped or not: a new wreck is counted and the light bars advance.
fn car_after_step(car: &mut AiCar, police: &mut Police) {
    if car.take_wreck() {
        police.record_wreck(&car.name);
    }
    car.step_lights();
}
