//! The chase camera: a behaviour model built from the car's `CameraGameplayExternal` data.
//!
//! The 2012 executable's camera update was **not** read (see `docs/provenance/chameleon-camera.md`), so the shape of
//! the model comes from the names of the data's structures (`Yaw_spring_modification`, `Acceleration_movement`,
//! `Camera_Roll`, `Deceleration_Pitch_Change`, `Speed_based_height_change`, `High_Speed_Shake`, `Impact_shake`,
//! gradient adjustments) and the role of each number from its value and how it varies between cars. Every such
//! mapping is **[unconfirmed]**; `docs/formats/chameleon-camera.md` lists them one by one.
//!
//! One step per rendered frame, after the car has moved. Space: x forward, y left, z up.

use crate::effects::{AccelFilter, ImpactState};
use crate::math::{
    MPS_TO_MPH, WORLD_UP, add, cross, filter_weight, lerp, normalize_or, ramp, rotate_about, rotate_z, spring_angle,
    sub, vertical_fov_deg, wrap_angle,
};
use crate::motion::{CameraOptions, CameraPose, CarMotion};
use crate::params::{CarCameras, ExternalCamera, GradientAdjustments, ImpactShake};

/// The chase camera of one car. Build it from the car's data, call [`ChaseCamera::reset`] when the car (re)appears
/// and [`ChaseCamera::step`] once per frame.
#[derive(Debug, Clone)]
pub struct ChaseCamera {
    camera: ExternalCamera,
    impact_params: ImpactShake,
    gradient: GradientAdjustments,
    options: CameraOptions,
    /// The camera's heading about +z in radians (0 = looking along +x), smoothed by the yaw spring.
    heading: f32,
    heading_rate: f32,
    accel: AccelFilter,
    /// Extra distance along the heading from the acceleration movement, metres (negative = further back).
    back_offset: f32,
    pitch_deg: f32,
    roll_deg: f32,
    shake_phase: f64,
    impact: ImpactState,
    ready: bool,
}

impl ChaseCamera {
    pub fn new(cameras: &CarCameras, options: &CameraOptions) -> Self {
        Self {
            camera: cameras.external.clone(),
            impact_params: cameras.globals.impact_shake.clone(),
            gradient: cameras.gradient,
            options: options.clone(),
            heading: 0.0,
            heading_rate: 0.0,
            accel: AccelFilter::default(),
            back_offset: 0.0,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            shake_phase: 0.0,
            impact: ImpactState::default(),
            ready: false,
        }
    }

    /// Place the camera at its rest pose behind the car with no history, and return it.
    pub fn reset(&mut self, motion: &CarMotion) -> CameraPose {
        let m = motion.sanitized();
        self.heading = heading_of(m.forward).unwrap_or(0.0);
        self.heading_rate = 0.0;
        self.accel.reset(m.speed);
        self.back_offset = 0.0;
        self.pitch_deg = 0.0;
        self.roll_deg = 0.0;
        self.shake_phase = 0.0;
        self.impact.clear();
        self.ready = true;
        self.compose(&m)
    }

    /// Start an impact shake of `strength` (0..1) by hand; `CarMotion::impact` does the same each frame.
    pub fn add_impact(&mut self, strength: f32) {
        self.impact.trigger(strength, &self.impact_params);
    }

    /// Advance the camera by one frame and return its pose.
    pub fn step(&mut self, motion: &CarMotion) -> CameraPose {
        let m = motion.sanitized();
        if !self.ready {
            return self.reset(&m);
        }
        if m.dt > 0.0 {
            self.integrate(&m);
        }
        self.compose(&m)
    }

    /// The camera's current heading in radians about +z (0 = along +x, positive = towards +y).
    pub fn heading(&self) -> f32 {
        self.heading
    }

    fn integrate(&mut self, m: &CarMotion) {
        let dt = m.dt;
        let ext = &self.camera;
        let accel_mps2 = self.accel.update(m.speed, dt, self.options.accel_filter_s);
        let mph = m.speed * MPS_TO_MPH;

        // Heading: a critically damped spring towards the car's heading (the direction of travel in the air),
        // aimed a little ahead along the yaw rate, stiffer or softer with speed.
        let yaw = &ext.yaw_spring;
        let car_heading = heading_of(m.forward).unwrap_or(self.heading);
        let travel = if !m.grounded && m.speed > 1.0 { heading_of(m.velocity) } else { None };
        let aim = wrap_angle(travel.unwrap_or(car_heading) + yaw.lead_s * m.yaw_rate);
        let omega = lerp(yaw.rate_low, yaw.rate_high, ramp(m.speed, yaw.speed_min, yaw.speed_max)).max(0.1);
        (self.heading, self.heading_rate) = spring_angle(self.heading, self.heading_rate, aim, omega, dt);

        // Acceleration movement: the camera drops back when the car pulls away and closes in when it brakes.
        let a = &ext.accel_movement;
        let accel01 = (accel_mps2 * MPS_TO_MPH / a.full_accel.abs().max(1e-3)).clamp(-1.0, 1.0);
        let wanted_back = if accel01 >= 0.0 { -a.back_m.abs() * accel01 } else { a.forward_m.abs() * -accel01 };
        self.back_offset += (wanted_back - self.back_offset) * filter_weight(dt, a.smoothing_s);

        // Pitch: down when braking hard, a little when accelerating, never faster than the data's rate.
        let d = &ext.decel_pitch;
        let wanted_pitch = -d.max_deg * m.brake * ramp(m.speed, 0.0, 5.0) + a.pitch_deg * accel01.max(0.0);
        let limit = d.rate_deg_s.abs() * dt;
        let pitch_step = (wanted_pitch - self.pitch_deg) * filter_weight(dt, 1.0 / d.response.abs().max(1e-3));
        self.pitch_deg += pitch_step.clamp(-limit, limit);

        // Roll: into the turn, in proportion to steering and to speed up to the roll's full speed.
        let r = &ext.roll;
        let wanted_roll = m.steer * r.max_deg * ramp(mph, 0.0, r.full_speed_mph);
        self.roll_deg += (wanted_roll - self.roll_deg) * filter_weight(dt, r.smoothing_s);

        let shake = &ext.speed_movement.shake;
        self.shake_phase += f64::from(dt * self.options.shake_base_hz * shake.rate * std::f32::consts::TAU);
        self.impact.trigger(m.impact, &self.impact_params);
        self.impact.advance(dt, &self.impact_params);
    }

    /// The pose for the current state and the car where it is now.
    fn compose(&self, m: &CarMotion) -> CameraPose {
        let ext = &self.camera;
        let shake_cfg = &ext.speed_movement.shake;
        let mph = m.speed * MPS_TO_MPH;
        let speed01 = ramp(mph, 0.0, shake_cfg.full_speed_mph);

        let offset = [
            ext.position[0] + ext.speed_movement.back_at_speed * speed01 + self.back_offset,
            ext.position[1],
            lerp(ext.position[2], ext.height_at_speed, speed01),
        ];
        let position = add(m.position, rotate_z(offset, self.heading));
        let target = add(m.position, rotate_z(ext.focus, self.heading));
        let level = rotate_z([1.0, 0.0, 0.0], self.heading);
        let mut forward = normalize_or(sub(target, position), level);

        let shake01 = ramp(mph, shake_cfg.start_speed_mph, shake_cfg.full_speed_mph).powi(2);
        let shake_deg = shake_cfg.amplitude * self.options.shake_scale_deg * shake01;
        let phase = self.shake_phase;
        let shake = [
            shake_deg * phase.sin() as f32,
            shake_deg * (1.3 * phase + 1.0).sin() as f32,
            shake_deg * (0.7 * phase + 2.0).sin() as f32,
        ];
        let impact = self.impact.rotation_deg(&self.impact_params);

        let elevation_deg = m.forward[2].clamp(-1.0, 1.0).asin().to_degrees();
        let follow = self.gradient.pitch_follow[usize::from(elevation_deg < 0.0)];
        let pitch_deg = self.pitch_deg + follow * elevation_deg + impact[0] + shake[0];
        let yaw_deg = impact[1] + shake[1];
        let roll_deg = self.roll_deg + impact[2] + shake[2];

        let left = normalize_or(cross(WORLD_UP, forward), [0.0, 1.0, 0.0]);
        forward = rotate_about(forward, left, -pitch_deg.to_radians());
        forward = normalize_or(rotate_about(forward, WORLD_UP, yaw_deg.to_radians()), level);
        let left = normalize_or(cross(WORLD_UP, forward), left);
        let up = cross(forward, left);
        let up = normalize_or(rotate_about(up, forward, -roll_deg.to_radians()), WORLD_UP);

        let [low, high] = ext.fov_range_deg();
        CameraPose {
            position,
            forward,
            up,
            vertical_fov_deg: vertical_fov_deg(lerp(low, high, speed01), self.options.aspect),
            shake: shake01,
        }
    }
}

/// The heading of a vector about +z, or `None` when it is (nearly) vertical or zero.
fn heading_of(v: [f32; 3]) -> Option<f32> {
    let flat = v[0].hypot(v[1]);
    (flat > 1e-3).then(|| v[1].atan2(v[0]))
}
