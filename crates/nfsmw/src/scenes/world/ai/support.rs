//! Ground support by heat level: the heavy and leader options of each `pursuitsupport` row and the delay before the first
//! request (`docs/formats/pursuit-data.md` section 2.3, install-verified). The request's cars are spawned and driven in
//! [`session`] (the request's state, the placement, the timers) and [`spawn`] (one car on the road). The car model of each
//! role is named here, not in the library (`blackbox_pursuit::support::Car`). Spec: `docs/specs/ai-pursuit-support.md`.

use blackbox_pursuit::support::{Car, Option_, Strategy};

mod session;
mod spawn;

pub use session::Session;

/// The minimum seconds of pursuit before any support is requested, per heat level 1 to 10 (`MinimumSupportDelay`).
pub fn minimum_delay(level: u32) -> f32 {
    const DELAY: [f32; 10] = [60.0, 60.0, 60.0, 45.0, 35.0, 10.0, 60.0, 10.0, 10.0, 10.0];
    DELAY[(level as usize).clamp(1, 10) - 1]
}

/// The heavy options of a heat level, in the row's order. Chance is a percentage; duration in seconds.
pub fn heavy_options(level: u32) -> Vec<Option_> {
    let opt = |strategy, chance, duration| Option_ { strategy, chance, duration };
    match level.clamp(1, 10) {
        3 => vec![opt(Strategy::HeavyRoadblock, 5.0, 40.0), opt(Strategy::Ram, 10.0, 20.0)],
        4 => vec![opt(Strategy::Ram, 25.0, 30.0), opt(Strategy::HeavyRoadblock, 25.0, 20.0)],
        5 => vec![opt(Strategy::Ram, 25.0, 30.0), opt(Strategy::HeavyRoadblock, 40.0, 20.0)],
        6 => vec![opt(Strategy::Ram, 20.0, 30.0)],
        7 => vec![opt(Strategy::Ram, 30.0, 30.0)],
        8 => vec![opt(Strategy::Ram, 40.0, 20.0)],
        _ => Vec::new(),
    }
}

/// The percentage of big SUVs (`ChanceBigSUV`) of a heavy option of a level. The rows give it in brackets where it is
/// not zero; a row without a bracket is read as 0 (unconfirmed for heat 3).
pub fn big_suv_chance(level: u32, strategy: Strategy) -> f32 {
    match (level.clamp(1, 10), strategy) {
        (4 | 5, Strategy::Ram | Strategy::HeavyRoadblock) => 100.0,
        (6 | 7, Strategy::Ram) => 100.0,
        (8, Strategy::Ram) => 80.0,
        _ => 0.0,
    }
}

/// The leader options of a heat level: each with its priority chance (percent) and priority time (seconds), which the
/// priority check reads once per heat (`blackbox_pursuit::support::priority`). Row 10's cross option has a chance of 0 and
/// no strategy in the install's rows, so it is left out.
pub fn leader_options(level: u32) -> Vec<(Option_, f32, f32)> {
    let cross = |chance, duration| Option_ { strategy: Strategy::CrossFollow, chance, duration };
    match level.clamp(1, 10) {
        5 => vec![(cross(5.0, 2000.0), 10.0, 120.0)],
        6 => vec![(cross(5.0, 2000.0), 100.0, 60.0)],
        8 => vec![(cross(0.0, 500.0), 0.0, 0.0)],
        9 => vec![(cross(100.0, 2000.0), 100.0, 30.0)],
        _ => Vec::new(),
    }
}

/// The car model a support role takes, by name (the install's cop car names).
pub fn car_model(car: Car) -> &'static str {
    match car {
        Car::BigSuv => "copsuv",
        Car::Suv => "copsuvl",
        Car::Cross => "copcross",
        Car::Hench => "copsporthench",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_delays_are_the_install_values() {
        assert_eq!(minimum_delay(1), 60.0);
        assert_eq!(minimum_delay(4), 45.0);
        assert_eq!(minimum_delay(6), 10.0);
    }

    #[test]
    fn heat_three_offers_the_roadblock_first() {
        let options = heavy_options(3);
        assert_eq!(options[0].strategy, Strategy::HeavyRoadblock);
        assert_eq!(options[0].chance, 5.0);
    }

    #[test]
    fn the_first_two_heats_and_the_last_two_have_no_heavy_options() {
        assert!(heavy_options(1).is_empty());
        assert!(heavy_options(2).is_empty());
        assert!(heavy_options(10).is_empty());
    }

    #[test]
    fn the_big_suv_chance_is_the_rows_bracket() {
        assert_eq!(big_suv_chance(4, Strategy::Ram), 100.0);
        assert_eq!(big_suv_chance(8, Strategy::Ram), 80.0);
        assert_eq!(big_suv_chance(3, Strategy::Ram), 0.0, "no bracket in heat 3");
    }

    #[test]
    fn the_cross_leader_appears_from_heat_five_with_the_priority_roll() {
        assert!(leader_options(4).is_empty());
        let heat_five = leader_options(5);
        assert_eq!(heat_five.len(), 1);
        let (option, priority_chance, priority_time) = heat_five[0];
        assert_eq!(option.strategy, Strategy::CrossFollow);
        assert_eq!((option.chance, option.duration), (5.0, 2000.0));
        assert_eq!((priority_chance, priority_time), (10.0, 120.0));
    }

    #[test]
    fn each_support_role_maps_to_its_install_car_model() {
        assert_eq!(car_model(Car::BigSuv), "copsuv");
        assert_eq!(car_model(Car::Suv), "copsuvl");
        assert_eq!(car_model(Car::Cross), "copcross");
        assert_eq!(car_model(Car::Hench), "copsporthench");
    }
}
