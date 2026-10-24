//! The unit the HUD shows speed in: kilometres per hour (default) or miles per hour. Only the display changes; the
//! physics and the world keep metres per second. Read by the HUD copy (`hud/plugin.rs`), which sets `HudState::use_mph`
//! for every HUD look.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpeedUnits {
    /// Kilometres per hour.
    #[default]
    Metric,
    /// Miles per hour.
    Imperial,
}
names!(SpeedUnits, "metric or imperial", [(Self::Metric, "metric"), (Self::Imperial, "imperial")]);
impl SpeedUnits {
    pub const ALL: [Self; 2] = [Self::Metric, Self::Imperial];
    /// The units' name in the current language, as the options show it.
    pub fn label(self) -> String {
        let id = match self {
            Self::Metric => "options-speed-units-metric",
            Self::Imperial => "options-speed-units-imperial",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_units_round_trip_default_to_metric_and_reject_other_names() {
        assert_eq!(SpeedUnits::default(), SpeedUnits::Metric);
        assert_eq!(SpeedUnits::ALL.len(), 2);
        for units in SpeedUnits::ALL {
            assert_eq!(units.to_string().parse::<SpeedUnits>(), Ok(units));
        }
        assert_eq!("metric".parse::<SpeedUnits>(), Ok(SpeedUnits::Metric));
        assert_eq!("imperial".parse::<SpeedUnits>(), Ok(SpeedUnits::Imperial));
        for name in ["mph", "kmh", "Metric", "", "si"] {
            assert!(name.parse::<SpeedUnits>().is_err(), "{name:?} must not parse");
        }
    }

    #[test]
    fn labels_name_both_units() {
        assert_eq!(SpeedUnits::Metric.label(), "Metric (km/h)");
        assert_eq!(SpeedUnits::Imperial.label(), "Imperial (mph)");
    }
}
