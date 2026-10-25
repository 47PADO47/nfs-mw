//! One player tick, with shared-world contacts resolved before publishing its pose.

use blackbox_collision::CollisionWorld;
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::effect_feed;
use super::walls::Impact;
use super::{Drive, DriveInput, GOOD_SPOT_SECONDS, Spawn, SpawnRequest, WorldGround, clock, fall, trace};
use crate::input::ActionState;
use crate::scenes::world::{props::PropWorld, road, space};

/// Work to finish after every body's motion and contacts have run.
pub struct DriveStep {
    input: DriveInput,
    impact: Impact,
    delta: f32,
}

impl Drive {
    /// Sample input once per render frame; presses wait for the next physics tick.
    pub fn prepare_frame(&mut self, actions: &ActionState, dt: f32) {
        self.frame_input = self.remote_input.unwrap_or_else(|| DriveInput::from_actions(actions, self.wheel));
        if self.script.is_none() && self.cinematic_input.is_none() {
            self.presses.note(&self.frame_input);
        }
        // An interactive app's first frame also has dt=0, before the car has been placed.
        self.batch_run |= dt == 0.0 && self.needs_batch();
        if let Some(sim) = self.sim.as_mut() {
            sim.set_automatic(self.transmission.is_automatic() || self.cinematic_input.is_some());
            sim.set_wheel_options(self.wheel);
            sim.cinematic_control(self.cinematic_input.is_some());
        }
    }

    pub fn needs_batch(&self) -> bool {
        self.request.is_none() && self.sim.is_some() && !self.script_finished()
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = match self.batch_run && self.script_finished() {
            true => 1.0,
            false => alpha,
        };
    }

    /// Integrate exactly one tick. Frame time never reaches the vehicle integrator.
    pub fn step(
        &mut self,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) -> Option<DriveStep> {
        self.step_with_delta(clock::STEP, collision, props, surfaces)
    }

    /// One shared tick with its simulation delta; rendering and real-time cadence are owned by the world clock.
    pub fn step_with_delta(
        &mut self,
        dt: f32,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) -> Option<DriveStep> {
        if !dt.is_finite() || dt <= 0.0 {
            return None;
        }
        if self.request.is_some() || (self.batch_run && self.script_finished()) {
            return None;
        }
        let sim = self.sim.as_mut()?;
        sim.cinematic_control(self.cinematic_input.is_some());
        sim.set_automatic(self.transmission.is_automatic() || self.cinematic_input.is_some());
        let input = match self.script.as_mut() {
            Some(run) => {
                let input = run.script.input_after(run.time, run.previous_time);
                run.previous_time = Some(run.time);
                run.time += dt;
                input
            }
            None => self.presses.take_into(self.frame_input),
        }
        .cinematic(self.cinematic_input)
        .during_arrest(self.arrested)
        .on_grid(self.gridded)
        .sabotaged(self.sabotaged);
        let assist = self.assist_aim.filter(|_| self.cinematic_input.is_none() && !self.arrested);
        let input = Self::assisted(assist, sim, input);
        if input.pop {
            self.flames.pop();
        }
        let impact =
            sim.step_with_delta(dt, &input, &WorldGround { collision, surfaces }, Some((collision, &mut *props)));
        let hits = impact
            .visuals
            .iter()
            .map(|v| (v.prop, v.info))
            .chain(impact.knocked.iter().map(|&(id, _)| (Some(id), None)));
        self.markers.log_new_hits(hits, props);
        Some(DriveStep { input, impact, delta: dt })
    }

    /// Read the final state after car/prop contacts, then emit effects, telemetry and traces.
    pub fn finish_step(&mut self, collision: &CollisionWorld, surfaces: &SurfaceTable, step: DriveStep) {
        let DriveStep { input, impact, delta: dt } = step;
        self.steer_lamp = input.steer;
        self.lamp_time += dt;
        let Some(sim) = self.sim.as_mut() else { return };
        self.brake_lamp = sim.lamp_brake();
        let (strength, speed, slip) = sim.drift_charge();
        self.breaker.charge_drift(strength, speed, slip, dt);
        self.pending_knocks.extend(impact.knocked.iter().map(|&(id, _)| id));
        let tire_contacts = sim.tire_contacts(collision);
        self.effects.step(tire_contacts, sim.effect_velocity(), dt);
        self.vehicle_effects.step(&impact.visuals, sim.pose(), sim.effect_velocity(), self.telemetry.nos_burning, dt);
        self.vehicle_effects.water_step(sim.water_contacts(tire_contacts), sim.pose(), sim.effect_velocity(), dt);
        self.vehicle_effects.bounce(collision, dt);
        self.markers.record(&impact.contacts, sim.tyre_hits());
        self.prop_touch.step(&impact.contacts, dt);
        self.previous = self.current;
        self.current = sim.pose();
        self.telemetry = sim.telemetry();
        let car = effect_feed::car_state(&self.current, &self.telemetry, sim.effect_velocity(), input.throttle);
        self.flames.step(dt, &car);
        self.tail_trails.step(car.to_world, car.nitrous, sim.effect_velocity().length());
        self.sound.after_step(sim, surfaces, &impact, &input, dt);
        self.chase.impact(sim.camera_acceleration());
        self.steps += 1;
        if let Some(run) = self.script.as_ref().filter(|_| self.steps.is_multiple_of(60)) {
            trace::log(
                run.time,
                (&self.effects, &self.vehicle_effects, &self.flames),
                (&self.current, &self.telemetry),
                &input,
            );
        }
        let finite = sim.is_finite();
        let reset_allowed = !sim.reset_collision_active();
        if self.batch_run && self.script_finished() {
            log::info!("tire effects at script end: {}", self.effects.status());
        }
        if input.reset && reset_allowed {
            self.manual_resets += 1;
            let at = self.current.position;
            log::info!("reset at ({:.0}, {:.0}, {:.0})", at.x, at.y, at.z);
            self.respawn_near([at.x, at.y], Some(self.heading()));
        }
        self.check_road(collision, finite, dt);
    }

    fn check_road(&mut self, collision: &CollisionWorld, finite: bool, dt: f32) {
        let wheels_down = self.telemetry.wheels_on_ground;
        let position = self.current.position;
        // Remember the last road the car stood on, and notice when it has fallen off the map.
        self.road_check_elapsed += f64::from(dt);
        if wheels_down >= 3 && self.road_check_elapsed >= GOOD_SPOT_SECONDS {
            self.road_check_elapsed = 0.0;
            if let Some(road::Surface { height, road: true }) =
                road::probe(collision, position.x, position.y, position.z + 1.5)
                && (height - position.z).abs() < 1.5
            {
                self.last_good =
                    Some(Spawn { position: Vec3::new(position.x, position.y, height), heading: self.heading() });
            }
        }
        let last_height = self.last_good.map(|g| g.position.z);
        let nothing_below = || {
            let [px, py, pz] = space::to_physics(position);
            collision.ground(px, pz, py, py - fall::PROBE).is_none()
        };
        if self.fall.update_with_delta(wheels_down, dt, position.z, last_height, nothing_below) {
            log::warn!(
                "the car fell off the map at ({:.0}, {:.0}, {:.0}); putting it back on the road",
                position.x,
                position.y,
                position.z
            );
            if !self.restore_last_good() {
                self.respawn_near([position.x, position.y], None);
            }
        }
        if !finite {
            log::error!("the car's state is not finite; putting it back on the road");
            if !self.restore_last_good() {
                self.request = Some(SpawnRequest {
                    near: [self.previous.position.x, self.previous.position.y],
                    heading: None,
                    exact: None,
                    strict: false,
                });
            }
        }
    }
}
