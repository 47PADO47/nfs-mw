//! The `[bike.burnout]` table: what the rear tyre does while the rider holds the front brake and the throttle with the bike standing
//! (`docs/specs/ride6-burnout.md`, `docs/modding/bikes.md`). Every key is optional: a bike without the table gets the road bikes' handler.

use blackbox_vehicle::bike::{BurnoutHelpSpec, BurnoutSpec};
use serde::Deserialize;

use super::ContentError;
use super::bike_check::ranged;

/// The burnout of a bike (`[bike.burnout]`).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Burnout {
    /// The factor on the rear tyre's longitudinal and lateral grip during a burnout (`BurnoutGripMultiplier`: 0.05 for the road bikes,
    /// 0.15 for the supermotos).
    pub grip: f32,
    /// The factor on the slip correction of the rear tyre (`BurnoutSlipCorrectionMultiplier`: the constructor's 0 for the road bikes, 0.9 for the supermotos).
    pub slip_correction: f32,
    /// A burnout needs the bike to be slower than this, km/h (`BurnoutMinSpeed`, the constructor's 5).
    pub speed_limit_kmh: f32,
    /// The lean limit while the burnout flag stands, radians (`BurnoutLeanLimit`: 18 degrees for the road bikes, 10 for the supermotos);
    /// 0 leaves the lean limit alone.
    pub lean_limit_rad: f32,
    /// Whether the traction control keeps its cut in a burnout (`bTCSLowSpeedModulationForBurnout`, the constructor's true).
    pub tcs_low_speed_modulation: bool,
    /// The burnout help force (`[bike.burnout.help]`).
    pub help: BurnoutHelp,
}

/// The burnout help force of a bike (`[bike.burnout.help]`): the yaw-rate controller that swings the rear round the held front wheel.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BurnoutHelp {
    /// `EnableBurnoutHelpForce`: true for the road bikes, false for the supermotos.
    pub enabled: bool,
    /// `BurnoutHelpForceFactor` (1).
    pub factor: f32,
    /// `BurnoutDesiredAngularSpeed` in rad/s (the data's 45 degrees per second): the yaw rate of the full stick.
    pub yaw_rate_rad_s: f32,
    /// `BurnoutSteerFilterTime` (s, 1).
    pub steer_filter_s: f32,
    /// `BurnoutHelpForceRPMPercentageRange`: the share of the rev limit over which the force comes in (0.75 to 0.95).
    pub rpm_range: [f32; 2],
    /// `BurnoutHelpReduction` (1), `BurnoutHelpReductionDuration` (s, 0.1) and `BurnoutHelpRaiseRate` (1/s, 10): what a collision does to the force.
    pub reduction: f32,
    pub reduction_s: f32,
    pub raise_rate: f32,
}

impl Default for BurnoutHelp {
    fn default() -> Self {
        let help = BurnoutHelpSpec::default();
        Self {
            enabled: help.enabled,
            factor: help.factor,
            yaw_rate_rad_s: help.yaw_rate,
            steer_filter_s: help.steer_filter,
            rpm_range: help.rpm_range,
            reduction: help.reduction,
            reduction_s: help.reduction_duration,
            raise_rate: help.raise_rate,
        }
    }
}

impl Default for Burnout {
    fn default() -> Self {
        let spec = BurnoutSpec::default();
        Self {
            grip: spec.grip,
            slip_correction: spec.slip_correction,
            speed_limit_kmh: spec.speed_limit * 3.6,
            lean_limit_rad: spec.lean_limit,
            tcs_low_speed_modulation: spec.tcs_modulation,
            help: BurnoutHelp::default(),
        }
    }
}

impl Burnout {
    /// The table as the dynamics take it.
    pub fn to_spec(self) -> BurnoutSpec {
        let help = self.help;
        BurnoutSpec {
            grip: self.grip,
            slip_correction: self.slip_correction,
            speed_limit: self.speed_limit_kmh / 3.6,
            lean_limit: self.lean_limit_rad,
            tcs_modulation: self.tcs_low_speed_modulation,
            help: BurnoutHelpSpec {
                enabled: help.enabled,
                factor: help.factor,
                yaw_rate: help.yaw_rate_rad_s,
                steer_filter: help.steer_filter_s,
                rpm_range: help.rpm_range,
                reduction: help.reduction,
                reduction_duration: help.reduction_s,
                raise_rate: help.raise_rate,
            },
        }
    }

    /// Check the ranges: they catch typing mistakes, not unusual bikes.
    pub(super) fn check(&self) -> Result<(), ContentError> {
        ranged(self.grip, "bike.burnout.grip", 0.0, 1.0)?;
        ranged(self.slip_correction, "bike.burnout.slip_correction", 0.0, 10.0)?;
        ranged(self.speed_limit_kmh, "bike.burnout.speed_limit_kmh", 0.1, 50.0)?;
        ranged(self.lean_limit_rad, "bike.burnout.lean_limit_rad", 0.0, std::f32::consts::FRAC_PI_2)?;
        let help = &self.help;
        ranged(help.factor, "bike.burnout.help.factor", 0.0, 100.0)?;
        ranged(help.yaw_rate_rad_s, "bike.burnout.help.yaw_rate_rad_s", 0.0, 20.0)?;
        ranged(help.steer_filter_s, "bike.burnout.help.steer_filter_s", 0.0, 100.0)?;
        ranged(help.rpm_range[0], "bike.burnout.help.rpm_range", -1.0, 5.0)?;
        ranged(help.rpm_range[1], "bike.burnout.help.rpm_range", -1.0, 5.0)?;
        ranged(help.reduction, "bike.burnout.help.reduction", 0.0, 1.0)?;
        ranged(help.reduction_s, "bike.burnout.help.reduction_s", 0.0, 10.0)?;
        ranged(help.raise_rate, "bike.burnout.help.raise_rate", 0.0, 1000.0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_road_bikes_handler() {
        let table = Burnout::default();
        assert_eq!((table.grip, table.slip_correction), (0.05, 0.0));
        assert!((table.speed_limit_kmh - 5.0).abs() < 1e-4);
        assert_eq!(table.to_spec(), BurnoutSpec::default());
    }

    #[test]
    fn a_table_with_one_key_keeps_the_defaults_of_the_rest() {
        let table: Burnout = toml::from_str("grip = 0.15\nslip_correction = 0.9").unwrap();
        assert_eq!((table.grip, table.slip_correction), (0.15, 0.9));
        assert!((table.speed_limit_kmh - 5.0).abs() < 1e-4);
        assert!(table.check().is_ok());
        assert!(toml::from_str::<Burnout>("grips = 1.0").is_err());
    }

    #[test]
    fn the_help_force_and_the_lean_limit_come_from_the_table_and_default_to_the_road_handlers() {
        let road = Burnout::default();
        assert!((road.lean_limit_rad - 18.0_f32.to_radians()).abs() < 1e-6 && road.tcs_low_speed_modulation);
        assert!(road.help.enabled && (road.help.yaw_rate_rad_s - 45.0_f32.to_radians()).abs() < 1e-6);
        assert_eq!((road.help.rpm_range, road.help.steer_filter_s), ([0.75, 0.95], 1.0));
        let motard: Burnout = toml::from_str(
            "lean_limit_rad = 0.1745
[help]
enabled = false
rpm_range = [0.5, 0.9]",
        )
        .unwrap();
        assert!(!motard.help.enabled && motard.help.rpm_range == [0.5, 0.9] && motard.help.factor == 1.0);
        let spec = motard.to_spec();
        assert!(!spec.help.enabled && (spec.lean_limit - 0.1745).abs() < 1e-6 && spec.is_valid());
        assert!(
            toml::from_str::<Burnout>(
                "[help]
force = 2"
            )
            .is_err()
        );
        assert!(Burnout { lean_limit_rad: 3.0, ..road }.check().is_err());
    }

    #[test]
    fn nonsense_numbers_are_refused() {
        assert!(Burnout { grip: 2.0, ..Burnout::default() }.check().is_err());
        assert!(Burnout { grip: f32::NAN, ..Burnout::default() }.check().is_err());
        assert!(Burnout { speed_limit_kmh: 0.0, ..Burnout::default() }.check().is_err());
    }
}
