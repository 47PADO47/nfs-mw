//! The unit the HUD shows speed in: kilometres per hour (default) or miles per hour. Only the display changes; the
//! physics and the world keep metres per second. Read by the HUD copy (`hud/plugin.rs`), which sets `HudState::use_mph`
//! for every HUD look.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

/// What miles per hour is in kilometres per hour, inverted: 1 km/h is this many mph (the HUD's 2.236936 per m/s is this times 3.6).
const MPH_PER_KMH: f32 = 0.621_371;

/// The units of the running game, for the places outside the HUD that name a speed in a text the player reads (the console log).
/// Set from the settings at start and on every change (`devtools::console::sync_settings`).
static IMPERIAL: AtomicBool = AtomicBool::new(false);

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

    /// Make these the units of the running game ([`SpeedUnits::current`]).
    pub fn make_current(self) {
        IMPERIAL.store(self == Self::Imperial, Ordering::Relaxed);
    }

    /// The units the player chose, as of the last [`SpeedUnits::make_current`] (metric before any).
    pub fn current() -> Self {
        match IMPERIAL.load(Ordering::Relaxed) {
            true => Self::Imperial,
            false => Self::Metric,
        }
    }

    /// A speed of `kmh` in these units, and the unit's short name: `"km/h"` or `"mph"`.
    pub fn of_kmh(self, kmh: f32) -> (f32, &'static str) {
        match self {
            Self::Metric => (kmh, "km/h"),
            Self::Imperial => (kmh * MPH_PER_KMH, "mph"),
        }
    }
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
    fn a_speed_is_given_in_the_chosen_units() {
        assert_eq!(SpeedUnits::Metric.of_kmh(180.0), (180.0, "km/h"));
        let (mph, name) = SpeedUnits::Imperial.of_kmh(225.0);
        assert_eq!(name, "mph");
        assert!((mph - 139.8).abs() < 0.05, "225 km/h is 139.8 mph: {mph}");
    }

    #[test]
    fn labels_name_both_units() {
        assert_eq!(SpeedUnits::Metric.label(), "Metric (km/h)");
        assert_eq!(SpeedUnits::Imperial.label(), "Imperial (mph)");
    }
}
