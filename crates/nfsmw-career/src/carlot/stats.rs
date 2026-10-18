//! The three stat bars of the car lot (`docs/specs/career-car-lot.md` open question 1, pass 5): TOP SPEED, HANDLING and
//! ACCELERATION of the shown car, each a fraction in [0, 1] of its bar.
//!
//! The spec's rule for a stock car (no upgrades, mask 0) is the car's base term `lo`, normalized over the whole lot:
//! `lo.x` from top speed, `lo.y` from handling and `lo.z = 1 - norm(time to 100 mph)`. The raw numbers come from the
//! caller (`nfsmw::career::data`), which measures each lot car once. This module only does the maths.

/// 100 mph in m/s, the time the ACCELERATION bar is measured to (`career-car-lot.md` open question 1: 44.703 m/s).
pub const SPEED_100_MPH: f32 = 44.703;

/// The speed step of the time integration, m/s.
const TIME_STEP: f32 = 0.05;

/// One lot car's raw stats, before normalization over the lot.
///
/// **[unconfirmed]** these come from the repo's estimator (`blackbox_vehicle::performance`), not from the game's own
/// simulation: top speed is the estimator's top speed, time to 100 mph is integrated from its acceleration table, and
/// handling is its start grip (a multiple of gravity). The spec's handling value is a lerp over attribute range
/// `0xAACBE2E7`, which is not reproduced here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarStats {
    /// Top speed, m/s.
    pub top_speed: f32,
    /// Handling: tire grip at rest, a multiple of `g`. **[unconfirmed]** stands in for the spec's handling value.
    pub handling: f32,
    /// Time from rest to [`SPEED_100_MPH`], seconds. `f32::INFINITY` when the car does not reach that speed.
    pub time_to_100: f32,
}

/// The time to reach `target` m/s from rest, given the acceleration `accel(speed)` at each speed and the car's
/// `top_speed`. Integrates `dt = dv / a` in steps of 0.05 m/s with the midpoint acceleration.
///
/// Returns `f32::INFINITY` when `target` is above `top_speed` or the acceleration is not positive on the way. A `target`
/// of 0 or less takes no time.
pub fn time_to_speed(target: f32, top_speed: f32, accel: impl Fn(f32) -> f32) -> f32 {
    if target > top_speed {
        return f32::INFINITY;
    }
    let mut speed = 0.0;
    let mut time = 0.0;
    while speed < target {
        let next = (speed + TIME_STEP).min(target);
        let a = accel(speed + (next - speed) / 2.0);
        if a.is_nan() || a <= 0.0 {
            return f32::INFINITY;
        }
        time += (next - speed) / a;
        speed = next;
    }
    time
}

/// The bar fraction of `value` between the population's `low` and `high`, clamped to [0, 1]. When the population has no
/// spread (`high` is not above `low`), the bar is half full: a lot of one car does not say whether it is good.
pub fn normalize(value: f32, low: f32, high: f32) -> f32 {
    let span = high - low;
    if span.is_nan() || span <= 0.0 {
        return 0.5;
    }
    ((value - low) / span).clamp(0.0, 1.0)
}

/// The lowest and highest of `values`, ignoring non-finite ones. `(0, 0)` when none is finite.
fn range(values: impl Iterator<Item = f32>) -> (f32, f32) {
    values
        .filter(|value| value.is_finite())
        .fold(None, |range: Option<(f32, f32)>, value| match range {
            None => Some((value, value)),
            Some((low, high)) => Some((low.min(value), high.max(value))),
        })
        .unwrap_or((0.0, 0.0))
}

/// The three bar fractions of `car` among `population` (every lot car's stats): `[TOP SPEED, HANDLING, ACCELERATION]`,
/// each in [0, 1].
///
/// Top speed and handling are normalized from the population's lowest to highest. ACCELERATION is `1 - norm(time to
/// 100 mph)`, so the quickest car fills its bar and a car that never reaches 100 mph (infinite time) empties it. This is
/// the spec's `lo` with mask 0 (`docs/specs/career-car-lot.md` open question 1), so a stock car's fill is `lo`.
pub fn bar_fractions(car: &CarStats, population: &[CarStats]) -> [f32; 3] {
    let (top_low, top_high) = range(population.iter().map(|stats| stats.top_speed));
    let (grip_low, grip_high) = range(population.iter().map(|stats| stats.handling));
    let (time_low, time_high) = range(population.iter().map(|stats| stats.time_to_100));
    let top = normalize(car.top_speed, top_low, top_high);
    let handling = normalize(car.handling, grip_low, grip_high);
    let time = match car.time_to_100.is_finite() {
        true => normalize(car.time_to_100, time_low, time_high),
        false => 1.0,
    };
    [top, handling, 1.0 - time]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(top_speed: f32, handling: f32, time_to_100: f32) -> CarStats {
        CarStats { top_speed, handling, time_to_100 }
    }

    #[test]
    fn normalize_maps_the_range_ends_to_zero_and_one_and_clamps() {
        assert_eq!(normalize(10.0, 10.0, 30.0), 0.0);
        assert_eq!(normalize(30.0, 10.0, 30.0), 1.0);
        assert!((normalize(20.0, 10.0, 30.0) - 0.5).abs() < 1e-6);
        assert_eq!(normalize(5.0, 10.0, 30.0), 0.0, "below the population is empty");
        assert_eq!(normalize(50.0, 10.0, 30.0), 1.0, "above the population is full");
    }

    #[test]
    fn a_population_with_no_spread_gives_a_half_bar() {
        assert_eq!(normalize(7.0, 7.0, 7.0), 0.5);
    }

    #[test]
    fn constant_acceleration_reaches_a_speed_in_speed_over_acceleration() {
        let time = time_to_speed(10.0, 50.0, |_| 2.0);
        assert!((time - 5.0).abs() < 0.01, "{time}");
    }

    #[test]
    fn a_car_that_never_reaches_the_target_takes_infinite_time() {
        assert_eq!(time_to_speed(SPEED_100_MPH, 30.0, |_| 5.0), f32::INFINITY, "top speed below 100 mph");
        assert_eq!(time_to_speed(10.0, 50.0, |_| 0.0), f32::INFINITY, "no acceleration");
    }

    #[test]
    fn a_target_of_zero_takes_no_time() {
        assert_eq!(time_to_speed(0.0, 50.0, |_| 2.0), 0.0);
    }

    #[test]
    fn the_fastest_car_fills_top_speed_and_the_quickest_fills_acceleration() {
        let lot = [stats(60.0, 1.0, 12.0), stats(80.0, 1.2, 8.0), stats(70.0, 1.1, 10.0)];
        let fast = bar_fractions(&lot[1], &lot);
        assert_eq!(fast, [1.0, 1.0, 1.0]);
        let slow = bar_fractions(&lot[0], &lot);
        assert_eq!(slow[0], 0.0);
        assert_eq!(slow[1], 0.0);
        assert_eq!(slow[2], 0.0);
    }

    #[test]
    fn a_car_between_the_ends_fills_by_its_place_in_the_range() {
        let lot = [stats(60.0, 1.0, 12.0), stats(80.0, 1.2, 8.0)];
        let middle = bar_fractions(&stats(70.0, 1.1, 10.0), &lot);
        assert!((middle[0] - 0.5).abs() < 1e-5, "{middle:?}");
        assert!((middle[1] - 0.5).abs() < 1e-5, "{middle:?}");
        assert!((middle[2] - 0.5).abs() < 1e-5, "{middle:?}");
    }

    #[test]
    fn a_car_that_never_reaches_100_mph_has_an_empty_acceleration_bar() {
        let lot = [stats(60.0, 1.0, f32::INFINITY), stats(80.0, 1.2, 8.0)];
        let slow = bar_fractions(&lot[0], &lot);
        assert_eq!(slow[2], 0.0);
    }

    #[test]
    fn a_lone_car_gets_half_bars() {
        let only = stats(70.0, 1.1, 10.0);
        assert_eq!(bar_fractions(&only, &[only]), [0.5, 0.5, 0.5]);
    }

    #[test]
    fn every_fraction_stays_in_the_unit_range() {
        let lot = [stats(60.0, 1.0, 12.0), stats(80.0, 1.2, 8.0)];
        let outside = stats(200.0, 9.0, 0.1);
        assert!(bar_fractions(&outside, &lot).iter().all(|f| (0.0..=1.0).contains(f)));
    }
}
