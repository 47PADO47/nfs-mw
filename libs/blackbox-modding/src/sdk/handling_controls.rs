//! The types of the controls hook of `bb.handling` 1.2 (catalogue: docs/modding/api-handling-controls.md).
//!
//! The hook is one closure, called once per physics tick for the vehicles the host applies hooks to. It names no aid: traction
//! control, ABS, stability control, a brake-bias map and a steering-feel model are all just a function of this query. Positive
//! steering, yaw rate and lateral acceleration are to the right; positive longitudinal acceleration is forward. Wheel arrays are in
//! the order front left, front right, rear left, rear right.

use super::Role;

/// What the host tells the hook about the driver and the car for one tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ControlsQuery {
    /// The kind of vehicle; `None` for a role this SDK does not know.
    pub role: Option<Role>,
    /// Changes whenever the vehicle changes: the `revision` of `Vehicles::spec`.
    pub vehicle_revision: u32,
    /// The tick length (s).
    pub dt: f32,
    /// The throttle request after the host's input shaping, 0 to 1.
    pub throttle: f32,
    /// The brake request after the host's input shaping (the idle hold included; pedals already swapped while reversing), 0 to 1.
    pub brake: f32,
    /// The handbrake request, 0 to 1.
    pub handbrake: f32,
    /// The steering request, -1 to 1, positive to the right.
    pub steer: f32,
    /// The speed along the vehicle's heading (m/s), negative while moving backwards.
    pub forward_speed: f32,
    /// Rotation about the body's up axis (rad/s, positive to the right).
    pub yaw_rate: f32,
    /// Acceleration along the body's lateral axis over the last tick (m/s^2, positive to the right).
    pub lateral_accel: f32,
    /// Acceleration along the vehicle's heading over the last tick (m/s^2, positive forward).
    pub longitudinal_accel: f32,
    /// Whether each wheel pressed on the ground in the last tick.
    pub wheel_loaded: [bool; 4],
    /// The slip ratio of each wheel in the last tick: (wheel surface speed - ground speed) / ground speed along the direction of
    /// travel, above 0 spinning, below 0 dragging, -1 locked; 0 for an airborne wheel.
    pub slip_ratio: [f32; 4],
    /// The slip angle of each wheel in the last tick, in turns (1.0 is 360 degrees), positive when the contact patch moves to the
    /// right; 0 for an airborne wheel.
    pub slip_angle: [f32; 4],
}

/// What the hook answers. Every number must be finite; the ranges are in the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ControlsResult {
    /// The throttle the engine is given, 0 to 1.
    pub throttle: f32,
    /// The brake demand of each wheel, 0 to 1.5, bias folded in; 1 is the full brake torque of that wheel. It replaces the host's
    /// brake split.
    pub brake: [f32; 4],
    /// The angle of the inside front wheel (rad, positive to the right); `None` leaves the steering to the host.
    pub steer_target_rad: Option<f32>,
}
