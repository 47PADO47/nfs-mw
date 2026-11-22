//! What the vehicle remembers from tick to tick for the external models of the suspension and the controls (the tyre model
//! needs nothing of the kind). Every field is written only while the model that reads it is installed, so a vehicle without
//! models never changes it.

use glam::Vec3;

/// The memory of one vehicle's model seams.
#[derive(Clone, Debug, Default)]
pub(super) struct HookMemory {
    /// The camber the suspension model answered for each wheel this tick (rad, outward positive); 0 when none answered. The
    /// tyre query of the wheel reads it.
    pub camber: [f32; 4],
    /// The velocity of the body point above each wheel in the last tick, for the acceleration the suspension model is told.
    pub point_velocity: Option<[Vec3; 4]>,
    /// The lateral force each wheel put on the body in the last tick (N, along the wheel's lateral axis, positive right).
    pub tyre_lateral_force: [f32; 4],
    /// The velocity of the body in the last controls update, for the accelerations the controls model is told.
    pub body_velocity: Option<Vec3>,
    /// The brake demand of each wheel the controls model answered this tick; `None` when none answered.
    pub brake_demand: Option<[f32; 4]>,
    /// The steering target (rad) the controls model answered this tick; `None` when it left the steering to the vehicle.
    pub steer_target: Option<f32>,
}
