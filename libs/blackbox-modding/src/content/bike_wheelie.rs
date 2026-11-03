//! `[bike.wheelie]`: the wheelie and stoppie assist and the rider help around it (`docs/specs/ride6-tricks.md`).

use blackbox_vehicle::bike::{PitchParams, WheelieSpec};
use serde::Deserialize;

use super::bike_help::RiderHelp;

fn pairs(points: &[[f32; 2]]) -> Vec<(f32, f32)> {
    points.iter().map(|&[x, y]| (x, y)).collect()
}

/// One parameter set of the wheelie and stoppie assist (`[bike.wheelie.wheelie]`, `[bike.wheelie.stoppie]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pitch {
    pub deadzone: f32,
    pub max_angle_rad: f32,
    pub speed_ramp_mps: [f32; 2],
    #[serde(default)]
    pub help_speed: Vec<[f32; 2]>,
    pub lean_help: [f32; 2],
    pub lookahead_s: f32,
    pub min_speed_mps: f32,
    pub acceleration: f32,
    pub acceleration_time_s: f32,
    pub ground_range_m: [f32; 2],
    pub lean_limit_rad: f32,
    pub lean_pitch_range_rad: [f32; 2],
    /// The share of the road's inclination taken off the pitch, and the time scale of the target angle's filter (0: none).
    #[serde(default)]
    pub terrain_influence: f32,
    #[serde(default)]
    pub target_filter_timescale: f32,
}

impl Pitch {
    fn to_spec(&self) -> PitchParams {
        PitchParams {
            deadzone: self.deadzone,
            max_angle: self.max_angle_rad,
            speed_ramp: self.speed_ramp_mps,
            help_speed: pairs(&self.help_speed),
            lean_help: self.lean_help,
            lookahead: self.lookahead_s,
            min_speed: self.min_speed_mps,
            acceleration: self.acceleration,
            acceleration_time: self.acceleration_time_s,
            ground_range: self.ground_range_m,
            lean_limit: self.lean_limit_rad,
            lean_pitch_range: self.lean_pitch_range_rad,
            terrain_influence: self.terrain_influence,
            filter_timescale: self.target_filter_timescale,
        }
    }
}

/// The wheelie and stoppie assist and the rider help around it (`[bike.wheelie]`). A range of `[0, 0]` switches a part off.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wheelie {
    pub wheelie: Pitch,
    pub stoppie: Pitch,
    /// The lift angle in rad over which the throttle is cut, while the rider is not asking for a wheelie and while he is, and
    /// the lift speed in rad/s over which it is cut.
    #[serde(default)]
    pub throttle_pitch_rad: [f32; 2],
    #[serde(default)]
    pub forced_throttle_pitch_rad: [f32; 2],
    #[serde(default)]
    pub throttle_pitch_speed: [f32; 2],
    /// The same for the front brake against a stoppie.
    #[serde(default)]
    pub brake_pitch_rad: [f32; 2],
    #[serde(default)]
    pub brake_pitch_speed: [f32; 2],
    /// The rest of the rider help (`[bike.wheelie.help]`, `bike_help.rs`).
    #[serde(default)]
    pub help: RiderHelp,
}

impl Wheelie {
    pub(super) fn to_spec(&self) -> WheelieSpec {
        WheelieSpec {
            wheelie: self.wheelie.to_spec(),
            stoppie: self.stoppie.to_spec(),
            throttle_pitch: self.throttle_pitch_rad,
            forced_throttle_pitch: self.forced_throttle_pitch_rad,
            throttle_pitch_speed: self.throttle_pitch_speed,
            brake_pitch: self.brake_pitch_rad,
            brake_pitch_speed: self.brake_pitch_speed,
            help: self.help.to_spec(),
        }
    }
}
