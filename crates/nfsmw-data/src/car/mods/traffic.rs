//! The traffic side of a mod car (docs/modding/api-content.md "Cars", docs/modding/api-traffic.md): the patterns it appears
//! in and the numbers it appears with, in the units of the game's own traffic patterns.
//!
//! The game keeps one list of car types per neighbourhood pattern, each type with a `rate` (the density-weighted waiting
//! time before the type may be picked again), a `max_instances` and a `percent` share of the traffic slots
//! (`crate::traffic::PatternVehicle`). A mod traffic car gets a rule in the lists of the patterns it names (every pattern when it
//! names none), made from its weight by [`TrafficUnit::rate`] and [`TrafficUnit::percent`]: a weight of 20 is the rule of
//! the game's `traftaxi` in its default pattern (rate 6, 20 percent). The manager's density, population budget and
//! cop-share settings act on these rules exactly as on the game's own.

use super::{ModCar, find_in, published};

/// A weight of 1 waits this long before the type may be picked again; the rate falls as the weight rises.
const RATE_AT_ONE_PERCENT: f32 = 120.0;
/// The fastest and the slowest rate of a rule: the game's common cars wait 3, its rarest trucks 60.
const RATE_RANGE: (f32, f32) = (2.0, 60.0);

/// What kind of road car a mod traffic car is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrafficClass {
    Compact,
    #[default]
    Sedan,
    Coupe,
    Suv,
    Pickup,
    Van,
    Truck,
    Bus,
    Taxi,
}

impl TrafficClass {
    /// The name the manifest and the devtools use.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Sedan => "sedan",
            Self::Coupe => "coupe",
            Self::Suv => "suv",
            Self::Pickup => "pickup",
            Self::Van => "van",
            Self::Truck => "truck",
            Self::Bus => "bus",
            Self::Taxi => "taxi",
        }
    }

    /// The `pvehicle` whose sound class (engine bank, horn and drive-by whoosh) a car of the class uses. A mod car has no
    /// record of its own, so the class chooses among the game's traffic cars; nothing else is taken from them.
    pub const fn sound_record(self) -> &'static str {
        match self {
            Self::Compact => "trafha",
            Self::Sedan => "traf4dseda",
            Self::Coupe => "trafficcoup",
            Self::Suv => "trafstwag",
            Self::Pickup => "trafpickupa",
            Self::Van => "trafminivan",
            Self::Truck | Self::Bus => "trafdmptr",
            Self::Taxi => "traftaxi",
        }
    }
}

/// How a mod car joins the traffic.
#[derive(Debug, Clone, PartialEq)]
pub struct TrafficUnit {
    pub class: TrafficClass,
    /// The spawn weight and share cap, percent, 1 to 100.
    pub weight: u32,
    /// At most this many cars of the kind at once; 0 is no limit.
    pub max_instances: u32,
    /// The factor on the pattern's cruising speeds.
    pub speed_factor: f32,
    /// The patterns the car appears in, lower case; empty is every pattern.
    pub patterns: Vec<String>,
}

impl TrafficUnit {
    /// The `rate` of the car's rule: 120 divided by the weight, between 2 and 60.
    pub fn rate(&self) -> f32 {
        (RATE_AT_ONE_PERCENT / self.weight.max(1) as f32).clamp(RATE_RANGE.0, RATE_RANGE.1)
    }

    /// The `percent` of the car's rule: its weight.
    pub fn percent(&self) -> u32 {
        self.weight.clamp(1, 100)
    }

    /// Whether the car appears in the pattern called `pattern` (any case).
    pub fn in_pattern(&self, pattern: &str) -> bool {
        self.patterns.is_empty() || self.patterns.iter().any(|name| name.eq_ignore_ascii_case(pattern))
    }

    /// A line for the devtools: `sedan, weight 20, no limit, x1, every pattern`.
    pub fn describe(&self) -> String {
        let limit = match self.max_instances {
            0 => "no limit".to_owned(),
            max => format!("max {max}"),
        };
        let places = match self.patterns.is_empty() {
            true => "every pattern".to_owned(),
            false => self.patterns.join(", "),
        };
        format!("{}, weight {}, {limit}, x{}, {places}", self.class.name(), self.weight, self.speed_factor)
    }
}

/// One rule of a traffic pattern, for a mod car: the units of `crate::traffic::PatternVehicle`.
#[derive(Debug, Clone, PartialEq)]
pub struct TrafficRule {
    /// The car's folder in lower case, which is how the pattern tables and the traffic models name cars.
    pub name: String,
    pub rate: f32,
    pub max_instances: u32,
    pub percent: u32,
}

/// The traffic unit of the mod car in `folder`. `None` for a stock car and for a mod car that is not traffic.
pub fn traffic_unit(folder: &str) -> Option<&'static TrafficUnit> {
    find_in(published(), folder).and_then(|car| car.traffic.as_ref())
}

/// Whether the car in `folder` is a mod car that is traffic.
pub fn is_traffic_car(folder: &str) -> bool {
    traffic_unit(folder).is_some()
}

/// The mod cars of this run that are traffic, in registration order.
pub fn traffic_cars() -> Vec<&'static ModCar> {
    published().iter().filter(|car| car.traffic.is_some()).collect()
}

/// How many traffic cars the mods have published so far. It changes when a rescan adds some, so the traffic manager knows
/// to read the rules again.
pub fn traffic_generation() -> usize {
    published().iter().filter(|car| car.traffic.is_some()).count()
}

/// The rules the mod traffic cars add to the pattern `pattern`, in registration order.
pub fn traffic_rules(pattern: &str) -> Vec<TrafficRule> {
    rules_in(published(), pattern)
}

fn rules_in(cars: &[ModCar], pattern: &str) -> Vec<TrafficRule> {
    cars.iter()
        .filter_map(|car| {
            let unit = car.traffic.as_ref().filter(|unit| unit.in_pattern(pattern))?;
            Some(TrafficRule {
                name: car.folder.to_ascii_lowercase(),
                rate: unit.rate(),
                max_instances: unit.max_instances,
                percent: unit.percent(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::{CarSpec, register};
    use super::*;
    use blackbox_carparts::CarTypeInfo;

    fn unit(weight: u32, patterns: &[&str]) -> TrafficUnit {
        TrafficUnit {
            class: TrafficClass::Sedan,
            weight,
            max_instances: 0,
            speed_factor: 1.0,
            patterns: patterns.iter().map(|name| (*name).to_owned()).collect(),
        }
    }

    fn cars_with(units: &[(&str, Option<TrafficUnit>, bool)]) -> Vec<ModCar> {
        let specs: Vec<CarSpec> = units
            .iter()
            .map(|(folder, traffic, player)| CarSpec {
                folder: (*folder).to_owned(),
                owner: "example.pack".to_owned(),
                name: None,
                manufacturer: None,
                player: *player,
                cop: None,
                traffic: traffic.clone(),
                geometry_in_mod: true,
                stats: Default::default(),
                engine_sound: None,
                car_effects: None,
            })
            .collect();
        let stock: Vec<CarTypeInfo> = Vec::new();
        register(&stock, &specs).0
    }

    #[test]
    fn a_weight_of_twenty_is_the_rule_of_the_games_taxi() {
        let taxi = unit(20, &[]);
        assert_eq!((taxi.rate(), taxi.percent()), (6.0, 20));
    }

    #[test]
    fn rare_cars_wait_longer_and_every_rate_stays_in_the_games_range() {
        assert!(unit(5, &[]).rate() > unit(40, &[]).rate());
        assert_eq!(unit(100, &[]).rate(), 2.0);
        assert_eq!(unit(1, &[]).rate(), 60.0);
        assert_eq!(unit(0, &[]).rate(), 60.0, "a weight of 0 is not divided by");
    }

    #[test]
    fn a_car_with_no_pattern_is_in_all_of_them() {
        let any = unit(20, &[]);
        assert!(any.in_pattern("downtown") && any.in_pattern("default"));
        let some = unit(20, &["downtown", "collegesouth"]);
        assert!(some.in_pattern("Downtown") && some.in_pattern("collegesouth"));
        assert!(!some.in_pattern("coastalnorth"));
    }

    #[test]
    fn the_rules_of_a_pattern_are_the_traffic_cars_that_appear_there() {
        let cars = cars_with(&[
            ("HATCH", Some(unit(20, &[])), false),
            ("TAXIMOD", Some(unit(10, &["downtown"])), false),
            ("RACER", None, true),
        ]);
        let names = |pattern: &str| rules_in(&cars, pattern).into_iter().map(|rule| rule.name).collect::<Vec<_>>();
        assert_eq!(names("downtown"), ["hatch", "taximod"]);
        assert_eq!(names("coastalnorth"), ["hatch"]);
        let rule = &rules_in(&cars, "downtown")[1];
        assert_eq!((rule.rate, rule.percent, rule.max_instances), (12.0, 10, 0));
    }

    #[test]
    fn a_car_that_is_traffic_only_is_a_traffic_type_of_the_table() {
        let cars = cars_with(&[("HATCH", Some(unit(20, &[])), false), ("BOTH", Some(unit(20, &[])), true)]);
        assert_eq!(cars[0].car_type.usage_type, 2, "the game's own traffic usage");
        assert_eq!(cars[1].car_type.usage_type, 0, "a player car that is also traffic stays a racing type");
    }

    #[test]
    fn the_unit_describes_itself() {
        assert_eq!(unit(20, &[]).describe(), "sedan, weight 20, no limit, x1, every pattern");
        let mut truck = unit(6, &["cityhighway"]);
        truck.max_instances = 1;
        assert_eq!(truck.describe(), "sedan, weight 6, max 1, x1, cityhighway");
    }
}
