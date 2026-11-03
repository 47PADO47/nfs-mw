//! What a step of the powertrain is given.

/// What a step of the powertrain is given.
#[derive(Clone, Copy, Debug)]
pub struct DriveInput {
    pub throttle: f32,
    /// The spin of the rear wheel (rad/s).
    pub wheel_spin: f32,
    /// The road speed (m/s).
    pub speed: f32,
    /// The front brake lever the rider asks for (0 to 1), before the linking of the brakes.
    pub front_brake: f32,
    /// The rear tyre's load over its load at rest.
    pub rear_load: f32,
    /// The effective radius of the rear wheel (m) and the lean as a share of the lean limit (0 upright, 1 at the limit).
    pub radius: f32,
    pub lean: f32,
    /// How much the traction control cuts the throttle, 0 to 1.
    pub traction_cut: f32,
    /// Whether the bike is in a burnout (`burnout.rs`): the launch clutch is not worked and the over-limit upshift does not come.
    pub burnout: bool,
    /// The rear wheel's inertia (kg m^2), the torque the road puts against its spin (the tyre's longitudinal force times the radius, N m,
    /// positive when driving the bike) and the brake torque on it (N m), from the last step.
    pub wheel_inertia: f32,
    pub traction_torque: f32,
    pub brake_torque: f32,
    pub dt: f32,
}
