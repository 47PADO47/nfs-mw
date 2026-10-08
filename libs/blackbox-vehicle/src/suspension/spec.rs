use crate::math::{LB_IN_TO_N_M, inch_to_m};

/// Chassis parameters (the attribute class `chassis`). Per-axle arrays are `[front, rear]`.
#[derive(Clone, Debug)]
pub struct ChassisSpec {
    /// Spring rate per wheel in lb/in.
    pub spring_stiffness: [f32; 2],
    /// Progressive stiffening of the spring, 1/m (the force is multiplied by `1 + c * progression`).
    pub spring_progression: [f32; 2],
    /// Damping while compressing, lb*s/in.
    pub shock_stiffness: [f32; 2],
    /// Damping while extending, lb*s/in.
    pub shock_ext_stiffness: [f32; 2],
    /// Shock speed threshold of the digressive valve, inches (used as m/s after conversion).
    pub shock_valving: [f32; 2],
    /// 0..1: how much the damper force flattens beyond the valving speed (0 = linear).
    pub shock_digression: [f32; 2],
    /// Damper force above this many body weights is blown off (0 disables).
    pub shock_blowout: f32,
    /// Anti-roll bar stiffness, lb/in on the left-right compression difference.
    pub swaybar_stiffness: [f32; 2],
    /// Suspension travel in inches.
    pub travel: [f32; 2],
    /// Ride height (wheel contact below the body box) in inches.
    pub ride_height: [f32; 2],
    /// Distance between the outer tire edges, metres.
    pub track_width: [f32; 2],
    /// Metres.
    pub wheel_base: f32,
    /// Front axle z in the body frame, metres.
    pub front_axle: f32,
    /// Percentage of the weight on the front axle.
    pub front_weight_bias: f32,
    /// Roll centre height above the road, inches; the centre of gravity height follows it.
    pub roll_center: f32,
}

/// One axle's constants converted to SI, for a given ride-height tuning.
#[derive(Clone, Copy, Debug)]
pub struct AxleConstants {
    pub spring: f32,
    pub shock: f32,
    pub shock_ext: f32,
    pub sway: f32,
    pub travel: f32,
    pub ride: f32,
    pub progression: f32,
    pub valving: f32,
    pub digression: f32,
}

impl ChassisSpec {
    /// Constants of `axle` (0 front, 1 rear). `ride_extra` is the ride-height tuning in inches.
    pub fn axle(&self, axle: usize, ride_extra: f32) -> AxleConstants {
        AxleConstants {
            spring: self.spring_stiffness[axle] * LB_IN_TO_N_M,
            shock: self.shock_stiffness[axle] * LB_IN_TO_N_M,
            shock_ext: self.shock_ext_stiffness[axle] * LB_IN_TO_N_M,
            sway: self.swaybar_stiffness[axle] * LB_IN_TO_N_M,
            travel: inch_to_m(self.travel[axle]),
            ride: inch_to_m(self.ride_height[axle] + ride_extra),
            progression: self.spring_progression[axle],
            valving: inch_to_m(self.shock_valving[axle]),
            digression: 1.0 - self.shock_digression[axle],
        }
    }
}
