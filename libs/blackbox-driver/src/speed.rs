//! The speed a racer or cop asks for: the curvature limit, the potential speed and acceleration, and the
//! governor that makes the request follow what the car can really do.
//! Spec: `docs/specs/ai-driver-speed-skill.md` (§3 to §5).

use glam::Vec2;

use crate::lookup::ramp;

/// Gravity used by the cornering limit.
const G: f32 = 9.8;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// What the car can do, after matching it to the weakest player car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarLimits {
    pub top_speed: f32,
    /// Tire grip (a multiple of `g`) at rest and at top speed.
    pub start_grip: f32,
    pub end_grip: f32,
    /// `aivehicle` `TopSpeedMultiplier` and `AccelerationMultiplier`.
    pub top_speed_multiplier: f32,
    pub acceleration_multiplier: f32,
}

/// The speed at which cornering a curve of curvature `kappa` just holds, capped at `top`.
pub fn speed_limit_for(kappa: f32, f0: f32, f1: f32, top: f32) -> f32 {
    let k = kappa.abs();
    let n = G * f1 + ((G * f1).powi(2) + 4.0 * k * G * f0).sqrt();
    n / (n / top.max(0.1)).max(2.0 * k)
}

/// How a cop's chase limits its speed near its target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PursuitContext {
    /// `MAXIMUM_AI_SPEED`, m/s (times 1.1 for a "jerk" target).
    pub distant: f32,
    /// The cop's offset from the target, planar.
    pub offset: Vec2,
    /// The unit vector from the target to where the path search was aimed.
    pub seek_dir: Vec2,
    pub target_speed: f32,
    /// The cop's unit forward vector and the steering directions of the two cars.
    pub forward: Vec2,
    pub steer_dir: Vec2,
    pub target_steer_dir: Vec2,
    /// A race is actively running (the speed attribute doubles) or the target is a "jerk" (x1.2).
    pub race_running: bool,
    pub jerk: bool,
}

const KMH: f32 = 1.0 / 3.6;

impl PursuitContext {
    /// The ceiling a cop feels: it slows down near a target that is slow or behind it.
    pub fn max_cop_speed(&self) -> f32 {
        let s = self.offset.dot(self.seek_dir);
        let ahead = s > 0.0;
        let fwd_near =
            (self.target_speed - (if ahead { 100.0 } else { 200.0 }) * KMH * 0.01 * s).clamp(10.0 * KMH, self.distant);
        let rev_near = match ahead {
            true => (-self.target_speed + s * 50.0 * KMH * 0.01).clamp(40.0 * KMH, self.distant),
            false => self.distant,
        };
        let near = lerp(rev_near, fwd_near, (self.forward.dot(self.seek_dir) + 0.5).clamp(0.0, 1.0));
        let side = 2.5 * (self.offset - self.seek_dir * s).length();
        let s = if ahead { s * 0.5 } else { s };
        let apparent = s.hypot(side);
        let near_scale = (1.0 - (apparent - 150.0) / 150.0).clamp(0.0, 1.0)
            * (self.steer_dir.dot(self.target_steer_dir).abs() + 0.2).clamp(0.0, 1.0);
        (near_scale * near + (1.0 - near_scale) * self.distant).clamp(0.0, self.distant)
    }
}

/// The speed the car could carry now: the cornering limit scaled by skill and the speed multiplier.
pub fn potential_speed(limits: &CarLimits, curvature: f32, skill: f32, pursuit: Option<&PursuitContext>) -> f32 {
    let f0 = limits.start_grip;
    let top = limits.top_speed;
    let cornering = match pursuit {
        Some(_) => speed_limit_for(curvature, f0, (limits.end_grip - f0) / top.max(0.1), top),
        None => {
            let scale = lerp(0.36, 0.9, skill);
            let f1 = (lerp(f0, limits.end_grip, scale) - f0) / top.max(0.1);
            speed_limit_for(curvature, f0, f1, top)
        }
    };
    let mut attribute = limits.top_speed_multiplier;
    if let Some(p) = pursuit {
        match () {
            _ if p.race_running => attribute *= 2.0,
            _ if p.jerk => attribute *= 1.2,
            _ => {}
        }
    }
    let potential = cornering * attribute * lerp(0.85, 1.0, skill);
    match pursuit {
        Some(p) => potential.min(p.max_cop_speed()),
        None => potential,
    }
}

/// The acceleration the car is expected to have: its table value scaled by multipliers and skill, less the
/// slope.
pub fn potential_acceleration(
    table_acceleration: f32,
    limits: &CarLimits,
    skill: f32,
    nos_boost: Option<f32>,
    slope_forward_y: f32,
) -> f32 {
    let nos = nos_boost.map_or(1.0, |b| b + 1.0);
    let scale = lerp(0.65, 1.0, skill);
    (table_acceleration * limits.acceleration_multiplier * nos * scale - G * 1.001 * slope_forward_y).max(0.0)
}

/// The requested speed, following the potential speed at the acceleration the car is expected to have.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpeedGovernor {
    pub speed_limit: f32,
    last_speed: f32,
}

impl SpeedGovernor {
    /// Starts at the car's current speed.
    pub fn begin(speed: f32) -> Self {
        Self { speed_limit: speed.max(0.0), last_speed: speed }
    }

    pub fn update(&mut self, speed: f32, potential_speed: f32, potential_accel: f32, skill: f32, dt: f32) -> f32 {
        let actual = (speed - self.last_speed) / dt.max(1e-3);
        self.last_speed = speed;
        if actual < 0.0 && self.speed_limit < potential_speed && self.speed_limit > speed {
            self.speed_limit += (potential_accel + actual).min(0.0) * dt;
        }
        if self.speed_limit < potential_speed {
            let t = ramp(self.speed_limit, 0.0, potential_speed);
            let e = lerp(1.5, 2.0, skill);
            let d = (potential_accel - actual * t.powf(e)).clamp(0.0, potential_accel);
            self.speed_limit += d * dt;
        }
        self.speed_limit = self.speed_limit.clamp(0.0, potential_speed);
        self.speed_limit
    }
}
