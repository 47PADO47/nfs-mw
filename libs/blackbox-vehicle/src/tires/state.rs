use crate::ground::SurfaceGrip;

/// Wheel moment of inertia (kg m^2), the same for every tire.
pub const WHEEL_INERTIA: f32 = 10.0;
/// Rolling friction (N m per rad/s) before the surface multiplier.
pub const ROLLING_FRICTION: f32 = 2.0;

/// Constants of one tire, built from the specs.
#[derive(Clone, Copy, Debug)]
pub struct TireParams {
    pub radius: f32,
    pub grip_scale: f32,
    pub static_grip: f32,
    pub dynamic_grip: f32,
    /// Full brake torque (N m).
    pub brake_spec: f32,
    /// Torque (N m) at which the brakes can lock the wheel.
    pub brake_lock_spec: f32,
    /// Full handbrake torque (N m).
    pub ebrake_spec: f32,
    /// Front (steered) wheels have no pilot factor.
    pub front: bool,
}

/// Inputs of a loaded-tire update.
#[derive(Clone, Copy, Debug)]
pub struct LoadedInput {
    /// Contact patch velocity in the wheel frame (m/s), sideways and forward.
    pub lat_vel: f32,
    pub fwd_vel: f32,
    /// Speed of the whole body (m/s).
    pub body_speed: f32,
    /// Vertical load on the tire (N).
    pub load: f32,
    pub dt: f32,
    /// A quarter of the vehicle mass (kg): bounds the low-speed brake force so it cannot reverse the car.
    pub quarter_mass: f32,
    pub surface: SurfaceGrip,
}

/// State and per-step modifiers of one tire. The tire carries no mass; the wheel inertia is a constant.
#[derive(Clone, Copy, Debug)]
pub struct Tire {
    /// Rotation speed (rad/s), + when rolling forward.
    pub av: f32,
    pub load: f32,
    pub lateral_force: f32,
    pub longitudinal_force: f32,
    /// Drive torque to apply this step (N m); the chassis sets it before the tire update.
    pub drive_torque: f32,
    /// 1 = gripping, below 1 = sliding.
    pub traction: f32,
    /// Wheel speed minus ground speed (m/s).
    pub slip: f32,
    /// In turns (1.0 = 360 degrees).
    pub slip_angle: f32,
    pub road_speed: f32,
    pub lateral_speed: f32,
    pub brake_locked: bool,
    pub(super) last_sign: i8,
    pub(super) last_torque: f32,
    pub(super) angular_acc: f32,
    // Per-step modifiers, reset by `begin_frame`.
    pub brake: f32,
    pub ebrake: f32,
    pub max_slip: f32,
    pub grip_boost: f32,
    pub traction_boost: f32,
    pub drag_reduction: f32,
    pub lateral_boost: f32,
    /// Lateral and longitudinal multipliers of the handling tuning.
    pub traction_circle: (f32, f32),
    pub drift_friction: f32,
}

impl Default for Tire {
    fn default() -> Self {
        Self {
            av: 0.0,
            load: 0.0,
            lateral_force: 0.0,
            longitudinal_force: 0.0,
            drive_torque: 0.0,
            traction: 1.0,
            slip: 0.0,
            slip_angle: 0.0,
            road_speed: 0.0,
            lateral_speed: 0.0,
            brake_locked: false,
            last_sign: 0,
            last_torque: 0.0,
            angular_acc: 0.0,
            brake: 0.0,
            ebrake: 0.0,
            max_slip: 0.5,
            grip_boost: 1.0,
            traction_boost: 1.0,
            drag_reduction: 0.15,
            lateral_boost: 1.0,
            traction_circle: (1.0, 1.0),
            drift_friction: 1.0,
        }
    }
}

impl Tire {
    /// Starts a step: clears torques, forces and the per-step multipliers, stores the step scales.
    pub fn begin_frame(&mut self, max_slip: f32, grip_boost: f32, traction_boost: f32) {
        self.drive_torque = 0.0;
        self.brake = 0.0;
        self.ebrake = 0.0;
        self.lateral_force = 0.0;
        self.longitudinal_force = 0.0;
        self.traction_circle = (1.0, 1.0);
        self.drift_friction = 1.0;
        self.max_slip = max_slip;
        self.grip_boost = grip_boost;
        self.traction_boost = traction_boost;
        self.drag_reduction = 0.15;
    }

    /// Stops the wheel speed from changing sign within a step: it is set to 0 instead, and the sign memory
    /// follows with a small dead band.
    pub(super) fn check_sign(&mut self) {
        const DEAD_BAND: f32 = 1e-6;
        if (self.last_sign > 0 && self.av < -DEAD_BAND) || (self.last_sign < 0 && self.av > DEAD_BAND) {
            self.av = 0.0;
        }
        self.last_sign = if self.av > DEAD_BAND {
            1
        } else if self.av < -DEAD_BAND {
            -1
        } else {
            0
        };
    }

    /// Rolling rate for display: 0 when locked, the ground rate when sliding on the ground, else the wheel speed.
    pub fn display_av(&self, radius: f32) -> f32 {
        if self.brake_locked {
            0.0
        } else if self.load > 0.0 && self.traction < 1.0 {
            self.road_speed / radius
        } else {
            self.av
        }
    }
}
