//! TEMPORARY stand-in for the vehicle physics until `blackbox-vehicle` lands.

use glam::{Quat, Vec3};
use nfsmw_data::car::WheelPose;

use super::clock::STEP;
use super::input::DriveInput;
use super::rig::CarPose;
use crate::scenes::world::road::Spawn;

#[derive(Debug, Clone, Copy, Default)]
pub struct Telemetry {
    pub speed_mps: f32,
    pub rpm: f32,
    pub gear: i32,
}

pub struct CarSim {
    position: Vec3,
    heading: f32,
    speed: f32,
    spin: f32,
}

impl CarSim {
    pub fn new(spawn: Spawn) -> Self {
        Self { position: spawn.position + Vec3::Z * 0.4, heading: spawn.heading, speed: 0.0, spin: 0.0 }
    }

    pub fn place(&mut self, spawn: Spawn) {
        *self = Self::new(spawn);
    }

    pub fn step(&mut self, input: &DriveInput) {
        self.speed += (input.throttle * 6.0 - input.brake * 10.0 - self.speed * 0.02) * STEP;
        self.heading -= input.steer * 0.6 * STEP * (self.speed / 10.0).min(1.0);
        self.position += Vec3::new(self.heading.cos(), self.heading.sin(), 0.0) * self.speed * STEP;
        self.spin += self.speed * STEP / 0.33;
    }

    pub fn pose(&self) -> CarPose {
        CarPose {
            position: self.position,
            rotation: Quat::from_rotation_z(self.heading),
            wheels: [WheelPose { steer: 0.0, spin: self.spin, travel: 0.0 }; 4],
        }
    }

    pub fn telemetry(&self) -> Telemetry {
        Telemetry { speed_mps: self.speed, rpm: 900.0 + self.speed * 60.0, gear: 1 }
    }
}
