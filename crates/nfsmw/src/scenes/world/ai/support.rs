//! Ground support by heat level: the heavy options of each `pursuitsupport` row and the delay before the first request
//! (`docs/formats/pursuit-data.md` section 2.3, install-verified). Only the roadblock strategy acts in the world; the RAM
//! and the leader strategies are rolled and logged, not carried out (their cars are not built).

use blackbox_pursuit::support::{Option_, Strategy};

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
}
