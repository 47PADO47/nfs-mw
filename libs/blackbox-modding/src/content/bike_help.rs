//! `[bike.wheelie.help]`: the rider help around a wheelie or a stoppie beyond the lift ranges of `[bike.wheelie]` itself: how
//! the throttle cut relaxes at a launch, which interventions act on the engine, and the auto-rider that moves the rider's
//! weight (`docs/specs/ride6-tricks.md`, sections 4 and 5). Every key is optional; a missing one takes the default of
//! `blackbox_vehicle::bike::RiderHelpSpec` / `AutoRiderSpec`.

use blackbox_vehicle::bike::{AutoRiderSpec, RiderHelpSpec};
use serde::Deserialize;

fn pairs(points: &[[f32; 2]]) -> Vec<(f32, f32)> {
    points.iter().map(|&[x, y]| (x, y)).collect()
}

/// The auto-rider (`[bike.wheelie.help.auto_rider]`).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AutoRider {
    pub wheelie_range_rad: Option<[f32; 2]>,
    pub stoppie_range_rad: Option<[f32; 2]>,
    pub multiplier: Option<f32>,
    pub pitch_speed_range: Option<[f32; 2]>,
    pub throttle_curve: Vec<[f32; 2]>,
    pub throttle_gain: Option<f32>,
    pub brake_curve: Vec<[f32; 2]>,
    pub brake_gain: Option<f32>,
    pub engage_speed: Option<f32>,
    pub manual_engage_speed: Option<f32>,
}

impl AutoRider {
    fn to_spec(&self) -> AutoRiderSpec {
        let base = AutoRiderSpec::default();
        AutoRiderSpec {
            wheelie_range: self.wheelie_range_rad.unwrap_or(base.wheelie_range),
            stoppie_range: self.stoppie_range_rad.unwrap_or(base.stoppie_range),
            multiplier: self.multiplier.unwrap_or(base.multiplier),
            pitch_speed_range: self.pitch_speed_range.unwrap_or(base.pitch_speed_range),
            throttle_curve: pairs(&self.throttle_curve),
            throttle_gain: self.throttle_gain.unwrap_or(base.throttle_gain),
            brake_curve: pairs(&self.brake_curve),
            brake_gain: self.brake_gain.unwrap_or(base.brake_gain),
            engage_speed: self.engage_speed.unwrap_or(base.engage_speed),
            manual_engage_speed: self.manual_engage_speed.unwrap_or(base.manual_engage_speed),
        }
    }
}

/// The rider help (`[bike.wheelie.help]`).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RiderHelp {
    pub launch_speed_mps: Option<f32>,
    pub launch_throttle_pitch_rad: Option<[f32; 2]>,
    pub launch_throttle_multiplier: Option<f32>,
    pub throttle_multiplier: Option<f32>,
    pub forced_brake_pitch_rad: Option<[f32; 2]>,
    pub brake_multiplier: Option<f32>,
    pub throttle_pitch_electronic: Option<bool>,
    pub throttle_speed_electronic: Option<bool>,
    pub brake_pitch_electronic: Option<bool>,
    pub brake_speed_electronic: Option<bool>,
    pub intensity: Option<f32>,
    pub weight_deadzone: Option<f32>,
    pub lookahead_s: Option<f32>,
    pub auto_rider: Option<AutoRider>,
}

impl RiderHelp {
    pub(super) fn to_spec(&self) -> RiderHelpSpec {
        let b = RiderHelpSpec::default();
        RiderHelpSpec {
            launch_speed: self.launch_speed_mps.unwrap_or(b.launch_speed),
            launch_throttle_pitch: self.launch_throttle_pitch_rad.unwrap_or(b.launch_throttle_pitch),
            launch_throttle_multiplier: self.launch_throttle_multiplier.unwrap_or(b.launch_throttle_multiplier),
            throttle_multiplier: self.throttle_multiplier.unwrap_or(b.throttle_multiplier),
            forced_brake_pitch: self.forced_brake_pitch_rad.unwrap_or(b.forced_brake_pitch),
            brake_multiplier: self.brake_multiplier.unwrap_or(b.brake_multiplier),
            throttle_pitch_electronic: self.throttle_pitch_electronic.unwrap_or(b.throttle_pitch_electronic),
            throttle_speed_electronic: self.throttle_speed_electronic.unwrap_or(b.throttle_speed_electronic),
            brake_pitch_electronic: self.brake_pitch_electronic.unwrap_or(b.brake_pitch_electronic),
            brake_speed_electronic: self.brake_speed_electronic.unwrap_or(b.brake_speed_electronic),
            intensity: self.intensity.unwrap_or(b.intensity),
            weight_deadzone: self.weight_deadzone.unwrap_or(b.weight_deadzone),
            lookahead: self.lookahead_s.unwrap_or(b.lookahead),
            auto_rider: self.auto_rider.as_ref().map(AutoRider::to_spec),
        }
    }
}
