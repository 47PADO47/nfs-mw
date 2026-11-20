//! The types of the suspension hook of `bb.handling` 1.1 (catalogue: docs/modding/api-handling-suspension.md).
//!
//! The hook is one closure, called once per axle per physics tick, the front axle first. It names no model: what it computes is
//! the mod's own. Compression is positive when a wheel moves up towards the body; a force on the body is positive when it pushes
//! the body up.

use super::Role;

/// What the host tells the hook about one wheel of an axle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SuspensionWheelQuery {
    /// Whether the wheel presses on the ground.
    pub loaded: bool,
    /// How far the wheel is pushed up from full droop (m), from 0 to `max_travel_m`; 0 when the wheel is not touching.
    pub compression_m: f32,
    /// The rate of change of the compression over the last tick (m/s, positive when compressing).
    pub compression_speed_mps: f32,
    /// The steering angle of this wheel (rad, positive to the right; 0 on an axle that does not steer).
    pub steer_angle_rad: f32,
    /// The acceleration of the body point above the wheel along the body's up axis (m/s^2), kinematic: 0 at rest, about -9.81 in
    /// free fall. Taken over the last tick; 0 on the first tick.
    pub body_accel_up_mps2: f32,
    /// The lateral force the tyre of this wheel put on the body in the last tick (N, positive to the right); 0 when airborne.
    pub tyre_lateral_force_n: f32,
    /// The vertical load the host estimates for this wheel at rest (N).
    pub static_load_n: f32,
}

/// What the host tells the hook about one axle for one tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SuspensionQuery {
    /// 0 front axle, 1 rear axle.
    pub axle: u32,
    /// The kind of vehicle; `None` for a role this SDK does not know.
    pub role: Option<Role>,
    /// Changes whenever the vehicle changes: the `revision` of `Vehicles::spec`.
    pub vehicle_revision: u32,
    /// The tick length (s).
    pub dt: f32,
    /// The compression of a wheel of this axle standing at rest on flat ground (m): the sag at which the vehicle's own spring carries
    /// the wheel's static load. `compression_m - rest_travel_m` is 0 for a car at rest.
    pub rest_travel_m: f32,
    /// The largest compression (m).
    pub max_travel_m: f32,
    /// The sprung mass the host attributes to one wheel (kg): a quarter of the vehicle mass.
    pub corner_mass_kg: f32,
    /// The left wheel, then the right wheel.
    pub wheels: [SuspensionWheelQuery; 2],
}

/// What the hook answers for one wheel.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SuspensionWheelResult {
    /// The force on the body at the wheel along the body's up axis (N, positive up); it may be negative.
    pub body_force_n: f32,
    /// The vertical force between the road and the tyre (N), at or above 0; it becomes the tyre's vertical load.
    pub tyre_load_n: f32,
    /// The camber of the wheel (rad, positive when the top leans away from the centre line of the car, on either side); the host
    /// hands it to the tyre hook of the same wheel in the same tick (`TyreQuery::camber_rad`).
    pub camber_rad: f32,
}

/// What the hook answers for one axle. Every number must be finite; the ranges are in the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SuspensionResult {
    /// The left wheel, then the right wheel.
    pub wheels: [SuspensionWheelResult; 2],
}
