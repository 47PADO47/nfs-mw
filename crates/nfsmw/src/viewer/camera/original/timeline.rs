//! Stable real-time updates with timestamped fixed-physics inputs.

use std::collections::VecDeque;

use glam::{Mat3, Quat, Vec3};
use nfsmw_data::camera::View;

use super::follow::{BodyInput, Follow, Sample};
use super::obstruction::{CameraHit, Obstruction};
use super::{anchor, impacts, noise, projection, underpass, vehicle_avoidance};

#[cfg(test)]
mod shake_tests;

#[derive(Debug, Clone, Copy, Default)]
pub struct NoiseControls {
    pub terrain: [f32; 2],
    pub brake: bool,
    pub drag: bool,
    pub overrev: bool,
}

#[derive(Clone)]
pub struct PhysicsFrame {
    pub body: BodyInput,
    /// Unobstructed custom view; angle is vertical FOV in binary-angle units.
    pub custom: Option<Sample>,
    pub avoidables: Vec<vehicle_avoidance::Avoidable>,
    pub vehicles: Vec<underpass::VehicleBounds>,
    pub noise: NoiseControls,
    pub chopper_positions: Vec<Vec3>,
    pub explosions: Vec<super::Explosion>,
    pub impacts: Vec<(impacts::Kind, f32)>,
}

pub const STEP: f64 = 1.0 / 60.0;
const EPSILON: f64 = 1e-7;

#[derive(Clone)]
struct BodyKey {
    time: f64,
    frame: PhysicsFrame,
}

#[derive(Clone, Copy)]
struct CameraKey {
    sample: Sample,
    body: BodyInput,
}

/// Frame timing comes from the world clock, before and after its ramp integral.
pub struct Frame<'a> {
    pub real_dt: f32,
    pub sim_dt: f32,
    pub view: Option<(usize, &'a View)>,
    pub custom: bool,
    pub aspect: f32,
    pub regions: Option<&'a blackbox_roads::TrackZones>,
    pub speedbreaker: bool,
    pub look_back: bool,
}

#[derive(Default)]
pub struct Timeline {
    follow: Follow,
    custom_breaker: super::custom::Breaker,
    obstruction: Obstruction,
    avoidance: vehicle_avoidance::Avoidance,
    noise: noise::Noise,
    underpass: underpass::Underpass,
    anchor: anchor::History,
    impacts: impacts::Impacts,
    shake: super::shake::Shake,
    consumed_time: f64,
    bodies: VecDeque<BodyKey>,
    real_time: f64,
    simulation_time: f64,
    physics_time: f64,
    camera_ticks: u64,
    zoom: f64,
    mode: Option<usize>,
    custom: bool,
    previous: Option<CameraKey>,
    current: Option<CameraKey>,
}

impl Timeline {
    /// Record every world step, including steps with no placed player.
    pub fn physics_step(&mut self, frame: Option<PhysicsFrame>) {
        self.physics_time += STEP;
        if let Some(frame) = frame {
            self.bodies.push_back(BodyKey { time: self.physics_time, frame });
        }
    }

    /// Seed a placement without changing the world's simulation clock.
    pub fn place(&mut self, frame: PhysicsFrame) {
        self.reset();
        self.bodies.clear();
        self.bodies.push_back(BodyKey { time: self.physics_time, frame });
    }

    pub fn reset(&mut self) {
        self.follow.reset();
        self.custom_breaker = Default::default();
        self.obstruction.reset();
        self.avoidance.reset();
        self.noise = noise::Noise::default();
        self.underpass.reset();
        self.anchor = anchor::History::default();
        self.impacts = impacts::Impacts::default();
        self.shake = super::shake::Shake::default();
        self.consumed_time = self.simulation_time;
        self.previous = None;
        self.current = None;
        self.mode = None;
    }

    pub fn advance(
        &mut self,
        frame: Frame<'_>,
        simulation_at: impl Fn(f32) -> f32,
        mut cast: impl FnMut(Vec3, Vec3) -> Option<CameraHit>,
        mut ground: impl FnMut(Vec3, Vec3, f32) -> Option<f32>,
    ) {
        if !frame.real_dt.is_finite() || frame.real_dt <= 0.0 {
            return;
        }
        let end = self.real_time + f64::from(frame.real_dt);
        if (frame.view.is_none() && !frame.custom || self.custom != frame.custom) && self.mode.is_some() {
            self.reset();
        }
        while (self.camera_ticks + 1) as f64 * STEP <= end + EPSILON {
            self.camera_ticks += 1;
            let offset = ((self.camera_ticks as f64 * STEP - self.real_time).max(0.0) as f32).min(frame.real_dt);
            let time = self.simulation_time + f64::from(simulation_at(offset));
            let (target, rate) = match frame.speedbreaker {
                true => (1.0, 4.0),
                false => (0.0, 1.0),
            };
            self.zoom += (target - self.zoom).clamp(-rate * STEP, rate * STEP);
            let mode = match frame.view {
                Some((mode, _)) => mode,
                None if frame.custom => 3,
                None => continue,
            };
            let Some(input) = self.body_at(time - STEP) else { continue };
            let mut body = input.body;
            // Consume each physical contact once, including intermediate contacts
            // in long frames and multiple camera ticks during slowed simulation.
            for key in &self.bodies {
                if key.time > self.consumed_time + EPSILON && key.time <= time + EPSILON {
                    for &(kind, speed) in &key.frame.impacts {
                        self.impacts.register(kind, speed);
                    }
                }
            }
            self.consumed_time = time;
            body.collision_damping = self.impacts.advance(STEP as f32)[0];
            body.acceleration = self.anchor.acceleration(body.position, body.forward, body.velocity, STEP as f32);
            let previous_eye = self.current.map_or(body.position, |key| key.sample.eye);
            if !frame.custom {
                self.shake.request(body.acceleration);
                self.shake.advance(STEP as f32);
                self.shake.explosions(&input.explosions, previous_eye, STEP as f32);
            }
            body.zoom = (1.0 - 0.4 * self.zoom) as f32;
            body.look_back = frame.look_back;
            body.up_influence = vehicle_avoidance::up_influence(body.position, body.velocity, &input.avoidables);
            let discontinuity = !frame.custom
                && (self.mode.is_none_or(|old| old != mode && (old < 2 || mode < 2))
                    || self.current.is_some_and(|key| key.body.look_back != body.look_back));
            let previous_forward = self
                .current
                .map_or(body.forward, |key| (key.sample.target - key.sample.eye).normalize_or(body.forward));
            let under = underpass::under_vehicle(0, body.position, body.velocity, previous_forward, &input.vehicles);
            self.underpass.advance(under, STEP as f32);
            let covered = frame.regions.is_some_and(|zones| super::covered(zones, previous_eye, body.position));
            let chopper_distances: Vec<f32> = input
                .chopper_positions
                .iter()
                .filter(|_| !covered)
                .map(|position| position.truncate().distance(previous_eye.truncate()))
                .collect();
            let (sample, suppress) = match frame.custom {
                true => {
                    let Some(sample) = input.custom else { continue };
                    (self.custom_breaker.apply(sample, body), false)
                }
                false => {
                    let Some((_, view)) = frame.view else { continue };
                    let sample = projection::finalize_sample(self.follow.update(mode, view, body, STEP as f32));
                    let (sample, suppress) = self.underpass.apply(mode, sample);
                    let sample = self.shake.apply(sample);
                    let sample = self.noise.apply(
                        mode,
                        sample,
                        noise::Input {
                            seconds: (self.camera_ticks as f64 * STEP) as f32,
                            speed: body.speed,
                            forward_acceleration: body.acceleration.dot(body.forward),
                            terrain: input.noise.terrain,
                            nos: body.nos_engaged,
                            brake: input.noise.brake,
                            drag: input.noise.drag,
                            overrev: input.noise.overrev,
                            chopper_distances: &chopper_distances,
                        },
                    );
                    (sample, suppress)
                }
            };
            let vertical = sample.fov_bangle;
            let sample = match frame.custom {
                true => super::custom::horizontal(sample, frame.aspect),
                false => sample,
            };
            let sample = self.obstruction.step_covered((mode, covered), body.position, sample, &mut cast, &mut ground);
            let mut sample = match suppress {
                true => sample,
                false => self.avoidance.step(mode, body.position, body.velocity, sample, &input.avoidables),
            };
            if frame.custom {
                sample.fov_bangle = vertical;
            }
            let key = CameraKey { sample, body };
            self.previous = match discontinuity {
                true => Some(key),
                false => self.current.or(Some(key)),
            };
            self.current = Some(key);
            self.mode = Some(mode);
            self.custom = frame.custom;
        }
        self.real_time = end;
        self.simulation_time += f64::from(frame.sim_dt);
        while self.bodies.len() > 2 && self.bodies[1].time <= self.simulation_time - STEP {
            self.bodies.pop_front();
        }
    }

    fn body_at(&self, time: f64) -> Option<PhysicsFrame> {
        let first = self.bodies.front()?;
        let low = self.bodies.iter().rev().find(|key| key.time <= time + EPSILON).unwrap_or(first);
        let high = self.bodies.iter().find(|key| key.time > low.time);
        let mut input = low.frame.clone();
        let Some(high) = high else { return Some(input) };
        let alpha = ((time - low.time) / (high.time - low.time)).clamp(0.0, 1.0) as f32;
        let next = high.frame.body;
        let previous = input.body;
        let rotation = Quat::from_mat3(&body_basis(previous)).slerp(Quat::from_mat3(&body_basis(next)), alpha);
        input.body.position = previous.position.lerp(next.position, alpha);
        input.body.forward = rotation * Vec3::X;
        input.body.up = rotation * Vec3::Z;
        input.body.velocity = previous.velocity.lerp(next.velocity, alpha);
        input.body.speed = previous.speed + (next.speed - previous.speed) * alpha;
        input.body.drift = previous.drift + (next.drift - previous.drift) * alpha;
        if let (Some(old), Some(new)) = (input.custom, high.frame.custom) {
            input.custom = Some(Sample {
                eye: input.body.position + (old.eye - previous.position).lerp(new.eye - next.position, alpha),
                target: input.body.position + (old.target - previous.position).lerp(new.target - next.position, alpha),
                up: old.up.lerp(new.up, alpha).normalize_or(Vec3::Z),
                fov_bangle: old.fov_bangle + (new.fov_bangle - old.fov_bangle) * alpha,
            });
        }
        Some(input)
    }

    pub fn sample(&self, body: BodyInput, interpolate: bool) -> Option<Sample> {
        let current = self.current?;
        let previous = self.previous.unwrap_or(current);
        let alpha = match interpolate {
            true => ((self.real_time - self.camera_ticks as f64 * STEP) / STEP).clamp(0.0, 1.0) as f32,
            false => 1.0,
        };
        let attached = self.mode.is_some_and(|mode| mode < 2);
        let offset = |key: CameraKey, point: Vec3| {
            let relative = point - key.body.position;
            match attached {
                true => body_basis(key.body).transpose() * relative,
                false => relative,
            }
        };
        let eye = offset(previous, previous.sample.eye).lerp(offset(current, current.sample.eye), alpha);
        let target = offset(previous, previous.sample.target).lerp(offset(current, current.sample.target), alpha);
        let (eye, target, up) = match attached {
            true => {
                let basis = body_basis(body);
                let up = (body_basis(previous.body).transpose() * previous.sample.up)
                    .lerp(body_basis(current.body).transpose() * current.sample.up, alpha);
                (basis * eye, basis * target, (basis * up).normalize_or(body.up))
            }
            false => (eye, target, previous.sample.up.lerp(current.sample.up, alpha).normalize_or(Vec3::Z)),
        };
        Some(Sample {
            eye: body.position + eye,
            target: body.position + target,
            up,
            fov_bangle: previous.sample.fov_bangle + (current.sample.fov_bangle - previous.sample.fov_bangle) * alpha,
        })
    }

    pub(super) fn is_custom(&self) -> bool {
        self.custom
    }

    #[cfg(test)]
    pub(super) fn ticks(&self) -> u64 {
        self.camera_ticks
    }

    #[cfg(test)]
    pub(super) fn collision_damping(&self) -> f32 {
        self.impacts.normalized()[0]
    }

    #[cfg(test)]
    pub(super) fn acceleration(&self) -> Vec3 {
        self.current.unwrap().body.acceleration
    }
}

fn body_basis(body: BodyInput) -> Mat3 {
    let forward = body.forward.normalize_or(Vec3::X);
    let left = body.up.cross(forward).normalize_or(Vec3::Y);
    Mat3::from_cols(forward, left, forward.cross(left).normalize_or(Vec3::Z))
}
