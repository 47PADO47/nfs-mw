//! Waiting for an oncoming gap before a car crosses another lane at a junction.
//! Rewrite extension (spec `docs/specs/ai-traffic-obstacles.md` §10.2). The original has no yield rule for traffic.

use glam::Vec2;

/// Seconds to the closest approach under which an oncoming car counts as a threat to the crossing.
pub const WAIT_FOR_GAP: f32 = 3.0;
/// Seconds of waiting after which the car goes anyway, so that the flow of traffic cannot deadlock.
pub const GIVE_UP_AFTER: f32 = 8.0;

/// Seconds until a car moving at `velocity` from `position` is closest to `point` (all on the road plane). `None` when
/// the car is standing still or is already moving away from the point.
pub fn time_to_closest(point: Vec2, position: Vec2, velocity: Vec2) -> Option<f32> {
    let speed_sq = velocity.length_squared();
    if speed_sq < 1e-6 {
        return None;
    }
    let t = (point - position).dot(velocity) / speed_sq;
    (t >= 0.0).then_some(t)
}

/// Whether the car may cross now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    Wait,
    Go,
}

/// The wait of one car at one crossing. It resets when the car goes.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Crossing {
    /// Seconds the car has waited at this crossing.
    pub waited: f32,
}

impl Crossing {
    /// Decides this think. `earliest` is the smallest [`time_to_closest`] of the oncoming cars, or `None` when no car
    /// is coming. The car waits while that is under [`WAIT_FOR_GAP`] and gives up after [`GIVE_UP_AFTER`] seconds.
    pub fn update(&mut self, dt: f32, earliest: Option<f32>) -> Gate {
        let threat = earliest.is_some_and(|t| t < WAIT_FOR_GAP);
        if !threat || self.waited >= GIVE_UP_AFTER {
            self.waited = 0.0;
            return Gate::Go;
        }
        self.waited += dt.max(0.0);
        Gate::Wait
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_car_coming_straight_at_the_point_has_a_time_to_closest_approach() {
        let t = time_to_closest(Vec2::new(0.0, 0.0), Vec2::new(0.0, 20.0), Vec2::new(0.0, -10.0)).unwrap();
        assert!((t - 2.0).abs() < 1e-5);
    }

    #[test]
    fn a_car_moving_away_or_standing_still_has_none() {
        assert_eq!(time_to_closest(Vec2::ZERO, Vec2::new(0.0, 20.0), Vec2::new(0.0, 10.0)), None);
        assert_eq!(time_to_closest(Vec2::ZERO, Vec2::new(0.0, 20.0), Vec2::ZERO), None);
    }

    #[test]
    fn a_car_passing_by_closest_in_the_past_has_none() {
        // Driving along +x, it passed the point at x = 0 behind it.
        assert_eq!(time_to_closest(Vec2::ZERO, Vec2::new(10.0, 5.0), Vec2::new(5.0, 0.0)), None);
    }

    #[test]
    fn a_car_coming_within_three_seconds_holds_the_crossing_until_it_has_passed() {
        let mut crossing = Crossing::default();
        assert_eq!(crossing.update(0.1, Some(2.9)), Gate::Wait);
        assert_eq!(crossing.update(0.1, Some(3.5)), Gate::Go, "a car at 3.5 s is no threat");
        assert_eq!(crossing.update(0.1, None), Gate::Go);
    }

    #[test]
    fn a_car_that_waits_too_long_goes_anyway_and_the_wait_starts_again() {
        let mut crossing = Crossing::default();
        let mut waits = 0;
        while crossing.update(0.5, Some(0.5)) == Gate::Wait {
            waits += 1;
        }
        assert_eq!(waits, 16, "8 s at 0.5 s a think");
        assert_eq!(crossing.waited, 0.0);
        assert_eq!(crossing.update(0.5, Some(0.5)), Gate::Wait, "the next crossing waits again");
    }
}
