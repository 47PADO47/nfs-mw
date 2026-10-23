//! The computer-driven world as a server replicates it (`docs/specs/multiplayer/world-host.md`): every car with its
//! entity id, kind, model, pose and motion after the last tick, and the helicopter. Poses are render space (x forward,
//! y left, z up, metres), the space the scene draws in. Nothing here changes the world.

use glam::{Quat, Vec3};

use super::TrafficWorld;

/// What a computer-driven car is, as a server sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetCarKind {
    /// A car cruising the streets (a trailer's tractor included).
    Traffic,
    /// A police car on patrol or in a chase.
    Police,
    /// A car of a standing roadblock, holding its place.
    Roadblock,
}

/// One computer-driven car after the last tick.
#[derive(Debug, Clone, Copy)]
pub struct NetCar<'a> {
    /// The car's entity id in its world (`AiCar::key`), stable for its life.
    pub entity: u32,
    pub kind: NetCarKind,
    /// The car model's folder name.
    pub model: &'a str,
    pub position: Vec3,
    pub rotation: Quat,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    /// The driver holds the brake.
    pub braking: bool,
    /// The light bar flashes.
    pub lights: bool,
    pub wrecked: bool,
}

/// The brake pedal from which the brake lamps count as lit [ours].
const BRAKING: f32 = 0.05;

impl TrafficWorld {
    /// Every car in entity-id order (the order they step in).
    pub fn net_cars(&self) -> impl Iterator<Item = NetCar<'_>> + '_ {
        self.cars.iter().map(|car| {
            let (pose, linear_velocity, angular_velocity) = car.stepped_motion();
            let kind = match (car.is_roadblock(), car.is_police()) {
                (true, _) => NetCarKind::Roadblock,
                (false, true) => NetCarKind::Police,
                (false, false) => NetCarKind::Traffic,
            };
            NetCar {
                entity: car.key(),
                kind,
                model: car.name.as_str(),
                position: pose.position,
                rotation: pose.rotation,
                linear_velocity,
                angular_velocity,
                braking: car.brake_pedal() > BRAKING,
                lights: car.emergency_lights(),
                wrecked: car.is_wrecked(),
            }
        })
    }

    /// The helicopter while one flies: its position, its frame (the nose along the way it faces; unturned when it has
    /// no direction) and its velocity, render space.
    pub fn net_helicopter(&self) -> Option<(Vec3, Quat, Vec3)> {
        let (position, forward, velocity) = self.helicopter_motion()?;
        let nose = forward.normalize_or_zero();
        let rotation = match nose == Vec3::ZERO {
            true => Quat::IDENTITY,
            false => Quat::from_rotation_arc(Vec3::X, nose),
        };
        Some((position, rotation, velocity))
    }
}
