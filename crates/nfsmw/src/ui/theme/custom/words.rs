//! The words and number formats the contextual layouts share: distances in the player's unit, clock times, grouped
//! whole numbers and the names of race types and places. Every word comes from the Fluent catalogue (`hud.ftl`).

use nfsmw_data::gameplay::RaceType;

use crate::hud::NavMarkerKind;
use crate::i18n::t;
use crate::settings::SpeedUnits;

const METRES_PER_MILE: f32 = 1609.344;
const FEET_PER_METRE: f32 = 3.280_84;

/// A distance with one decimal in the long unit: `4.2 km` or `2.6 mi`. The race strip's remaining distance.
pub fn distance_long(metres: f32, units: SpeedUnits) -> String {
    let metres = metres.max(0.0);
    match units {
        SpeedUnits::Metric => t!("hud-distance-km", value = format!("{:.1}", metres / 1000.0)),
        SpeedUnits::Imperial => t!("hud-distance-mi", value = format!("{:.1}", metres / METRES_PER_MILE)),
    }
}

/// A distance for a nearby place: metres (or feet) to the ten below a kilometre (a tenth of a mile), else the long
/// unit with one decimal.
pub fn distance_near(metres: f32, units: SpeedUnits) -> String {
    let metres = metres.max(0.0);
    let tens = |v: f32| ((v / 10.0).round() * 10.0) as u32;
    match units {
        SpeedUnits::Metric if metres < 1000.0 => t!("hud-distance-m", value = tens(metres)),
        SpeedUnits::Imperial if metres < METRES_PER_MILE / 10.0 => {
            t!("hud-distance-ft", value = tens(metres * FEET_PER_METRE))
        }
        _ => distance_long(metres, units),
    }
}

/// Seconds as `mm:ss`, minutes past an hour going on counting (`75:10`).
pub fn clock(secs: f32) -> String {
    let whole = secs.max(0.0) as u32;
    format!("{:02}:{:02}", whole / 60, whole % 60)
}

/// A whole number with the language's digit grouping (`12,500`).
pub fn grouped(value: u32) -> String {
    crate::i18n::fmt::grouped(i64::from(value))
}

/// The name of a race type.
pub fn race_type(kind: RaceType) -> String {
    match kind {
        RaceType::Sprint => t!("hud-race-type-sprint"),
        RaceType::Circuit => t!("hud-race-type-circuit"),
        RaceType::Drag => t!("hud-race-type-drag"),
        RaceType::Knockout => t!("hud-race-type-knockout"),
        RaceType::Tollbooth => t!("hud-race-type-tollbooth"),
        RaceType::SpeedTrap => t!("hud-race-type-speed-trap"),
        RaceType::Challenge => t!("hud-race-type-challenge"),
        _ => t!("hud-race-type-race"),
    }
}

/// The name of a kind of place on the map, as the entry cards name it.
pub fn place(kind: NavMarkerKind) -> String {
    match kind {
        NavMarkerKind::Race(kind) => race_type(kind),
        NavMarkerKind::SafeHouse => t!("hud-entry-safe-house"),
        NavMarkerKind::AutoShop => t!("hud-entry-auto-shop"),
        NavMarkerKind::CarLot => t!("hud-entry-car-lot"),
    }
}

/// The track's name for a career event id (`10.2.1` reads `TRACKNAME_10_2_1`), as the front end names tracks; the id
/// itself when the language has no such label.
pub fn track_name(event: &str) -> String {
    let key = format!("TRACKNAME_{}", event.replace('.', "_"));
    crate::i18n::game(blackbox_text::label_hash(&key)).unwrap_or_else(|| event.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_follow_the_unit() {
        assert_eq!(distance_long(4234.0, SpeedUnits::Metric), "4.2 km");
        assert_eq!(distance_long(4234.0, SpeedUnits::Imperial), "2.6 mi");
        assert_eq!(distance_near(246.0, SpeedUnits::Metric), "250 m");
        assert_eq!(distance_near(100.0, SpeedUnits::Imperial), "330 ft");
        assert_eq!(distance_near(1500.0, SpeedUnits::Metric), "1.5 km");
    }

    #[test]
    fn clocks_are_minutes_and_seconds() {
        assert_eq!(clock(134.7), "02:14");
        assert_eq!(clock(-3.0), "00:00");
        assert_eq!(clock(4510.0), "75:10");
    }
}
