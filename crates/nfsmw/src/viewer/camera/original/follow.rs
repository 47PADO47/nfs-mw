//! Retained original-view follow controller, implemented from the behavioral spec.

use glam::{Mat3, Vec3};
use nfsmw_data::camera::View;

use super::{
    spline::{Scalar, Vector},
    transients,
    tweaks::{self, Tweaks},
};

#[derive(Debug, Clone, Copy)]
pub struct BodyInput {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub speed: f32,
    pub top_speed: f32,
    pub grounded: bool,
    pub nos_engaged: bool,
    pub collision_damping: f32,
    pub drift: f32,
    pub up_influence: Vec3,
    pub zoom: f32,
    pub look_back: bool,
    pub gear: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_bangle: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Follow {
    eye: Vector,
    look: Vector,
    direction: Vector,
    up: Vector,
    fov: Scalar,
    acceleration: [Vec3; 5],
    acceleration_count: usize,
    acceleration_cursor: usize,
    acceleration_sum: Vec3,
    since_ground: f32,
    mode: Option<usize>,
    transients: transients::Transients,
}

impl Follow {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, mode: usize, view: &View, body: BodyInput, dt: f32) -> Sample {
        let outside = mode >= 2;
        let first = self.mode.is_none();
        let snap = first || self.mode.is_some_and(|old| old != mode && !(old >= 2 && outside));
        let tuning = Tweaks::at(mode, body.speed);
        let forward = body.forward.normalize_or(Vec3::X);
        if first {
            self.direction.target(forward);
            self.direction.snap();
            self.up.target(Vec3::Z);
            self.up.snap();
        }
        self.since_ground = match body.grounded {
            true => 0.0,
            false => self.since_ground + dt,
        };
        self.advance(dt, &tuning, body.collision_damping + body.drift);
        let basis = camera_basis(outside, self.direction.value(), forward, body.up);
        let mapped_acceleration = tweaks::acceleration(body.acceleration);
        let averaged = self.average(mapped_acceleration);
        let local_acceleration = basis.transpose() * averaged;
        let zoom = body.zoom.clamp(0.01, 1.0);
        let lag = view.lag[0] / zoom * tweaks::top_speed_scale(body.speed, body.top_speed);
        let height = view.height[0];
        let angle = f32::from(bangle(view.angle[0])) * (std::f32::consts::TAU / 65536.0);
        let eye = Vec3::new(lag, view.lat_offset[0], height) + tuning.eye.apply(local_acceleration);
        let look = match outside {
            true => Vec3::new(0.0, 0.0, height + angle.tan() * lag),
            false => Vec3::new(lag + 1.0, 0.0, height - angle.tan()),
        } + tuning.look.apply(local_acceleration);
        self.eye.target(eye);
        self.look.target(look);
        let mut fov = bangle(view.fov[0] * zoom);
        if outside && body.nos_engaged && body.speed > 0.0 {
            fov = fov.wrapping_add(1638);
        }
        self.fov.target(f32::from(fov));
        let direction_target = self.direction_target(outside, body, forward);
        self.direction.target(direction_target);
        if !outside || snap {
            self.direction.snap();
        }
        let mut up = Vec3::Z;
        if view.tilting[0] && outside {
            up = (averaged * tuning.up_scale + Vec3::Z).clamp(Vec3::new(-1.0, -1.0, 1.0), Vec3::ONE);
        }
        if body.speed > 5.0 {
            up += body.up_influence;
        }
        self.up.target(up.normalize_or(Vec3::Z));
        if snap || !outside {
            self.eye.snap();
            self.look.snap();
            self.fov.snap();
            self.up.snap();
        }
        self.mode = Some(mode);
        let basis = match snap {
            true => camera_basis(outside, self.direction.value(), forward, body.up),
            false => basis,
        };
        let reverse = match body.look_back {
            true => Vec3::new(-1.0, -1.0, 1.0),
            false => Vec3::ONE,
        };
        let up = match outside {
            true => Vec3::new(self.up.value().x, self.up.value().y, 1.0),
            false => body.up.normalize_or(Vec3::Z),
        };
        let (eye, target) = self.transients.apply(
            outside,
            self.eye.value() * reverse,
            self.look.value() * reverse,
            transients::Input {
                mapped_acceleration,
                collision_damping: body.collision_damping,
                speed: body.speed,
                gear: body.gear,
                dt,
            },
        );
        Sample {
            eye: body.position + basis * eye,
            target: body.position + basis * target,
            up,
            fov_bangle: self.fov.value,
        }
    }

    fn advance(&mut self, dt: f32, tuning: &Tweaks, extra_direction_duration: f32) {
        self.up.advance(dt, tuning.up_duration);
        self.fov.advance(dt, tuning.fov_duration);
        self.look.advance(dt, tuning.look_duration);
        self.direction.advance(dt, tuning.direction_duration + extra_direction_duration);
        self.eye.advance(dt, tuning.eye_duration);
    }

    fn average(&mut self, value: Vec3) -> Vec3 {
        self.acceleration_sum -= self.acceleration[self.acceleration_cursor];
        self.acceleration_sum += value;
        self.acceleration[self.acceleration_cursor] = value;
        self.acceleration_count = (self.acceleration_count + 1).min(5);
        self.acceleration_cursor = (self.acceleration_cursor + 1) % 5;
        self.acceleration_sum / self.acceleration_count as f32
    }

    fn direction_target(&self, outside: bool, body: BodyInput, forward: Vec3) -> Vec3 {
        if !outside {
            return forward;
        }
        let mut motion = body.velocity;
        let dot = motion.dot(forward);
        if dot < 0.0 {
            motion -= 2.0 * dot * forward;
        }
        let weight = ((body.speed - 8.0) / 5.0).clamp(0.0, 1.0);
        let mut direction = motion.normalize_or_zero() * weight + forward * (1.0 - weight);
        direction.z *= (1.0 - self.since_ground * 0.5).clamp(0.0, 1.0) * (1.0 - forward.z * forward.z);
        direction
    }
}

fn camera_basis(outside: bool, direction: Vec3, forward: Vec3, body_up: Vec3) -> Mat3 {
    let forward = match outside {
        true => direction.normalize_or(forward),
        false => forward,
    };
    let reference = match outside {
        true => Vec3::Z,
        false => match body_up.z < 0.0 {
            true => -body_up.normalize_or(Vec3::Z),
            false => body_up.normalize_or(Vec3::Z),
        },
    };
    let left = reference.cross(forward);
    let up = forward.cross(left);
    Mat3::from_cols(forward, left, up)
}

fn bangle(degrees: f32) -> u16 {
    ((degrees * 65536.0) as i32 / 360) as u16
}
