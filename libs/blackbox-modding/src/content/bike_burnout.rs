//! The `[bike.burnout]` table: what the rear tyre does while the rider holds the front brake and the throttle with the bike standing
//! (`docs/specs/ride6-burnout.md`, `docs/modding/bikes.md`). Every key is optional: a bike without the table gets the road bikes' handler.

use blackbox_vehicle::bike::BurnoutSpec;
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
}

impl Default for Burnout {
    fn default() -> Self {
        let spec = BurnoutSpec::default();
        Self { grip: spec.grip, slip_correction: spec.slip_correction, speed_limit_kmh: spec.speed_limit * 3.6 }
    }
}

impl Burnout {
    /// The table as the dynamics take it.
    pub fn to_spec(self) -> BurnoutSpec {
        BurnoutSpec { grip: self.grip, slip_correction: self.slip_correction, speed_limit: self.speed_limit_kmh / 3.6 }
    }

    /// Check the ranges: they catch typing mistakes, not unusual bikes.
    pub(super) fn check(&self) -> Result<(), ContentError> {
        ranged(self.grip, "bike.burnout.grip", 0.0, 1.0)?;
        ranged(self.slip_correction, "bike.burnout.slip_correction", 0.0, 10.0)?;
        ranged(self.speed_limit_kmh, "bike.burnout.speed_limit_kmh", 0.1, 50.0)?;
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
    fn nonsense_numbers_are_refused() {
        assert!(Burnout { grip: 2.0, ..Burnout::default() }.check().is_err());
        assert!(Burnout { grip: f32::NAN, ..Burnout::default() }.check().is_err());
        assert!(Burnout { speed_limit_kmh: 0.0, ..Burnout::default() }.check().is_err());
    }
}
