use glam::Vec3;

/// What a renderer or a sound system wants to know about one wheel.
#[derive(Clone, Copy, Debug)]
pub struct WheelState {
    /// World position of the tire contact patch (where the road would be at the current spring length).
    pub position: Vec3,
    pub radius: f32,
    /// Steering angle in radians (+ = right); 0 for the rear wheels.
    pub steer_angle: f32,
    /// Visual rotation rate (rad/s): 0 when locked, the rolling rate when sliding on the ground.
    pub angular_velocity: f32,
    /// Spring compression in metres.
    pub compression: f32,
    /// Pressed onto the ground with load.
    pub on_ground: bool,
    /// Vertical tire load in newtons.
    pub load: f32,
    /// Wheel speed minus ground speed in m/s (+ = spinning).
    pub slip: f32,
    /// Slip angle in radians.
    pub slip_angle: f32,
    /// 1 = gripping, below 1 = sliding.
    pub traction: f32,
    pub locked: bool,
    pub lateral_force: f32,
    pub longitudinal_force: f32,
    /// Speed (m/s) at which the contact patch slides over the road, combining wheel spin and sideways
    /// motion; 0 for a wheel in the air.
    pub slide_speed: f32,
    /// 0..1: how hard the tire is scrubbing the road. A renderer lays skid marks where this is above a
    /// small threshold, with opacity following it.
    pub skid: f32,
    /// 0..1: tire smoke. Rises later than skid marks and is strongest for burnouts, locked wheels and
    /// drifts.
    pub smoke: f32,
}

/// Slide speed (m/s) where skid marks start and where they are fully dark.
pub const SKID_RANGE: (f32, f32) = (2.0, 5.0);
/// Slide speed (m/s) where smoke starts and where it is densest.
pub const SMOKE_RANGE: (f32, f32) = (3.5, 9.0);
