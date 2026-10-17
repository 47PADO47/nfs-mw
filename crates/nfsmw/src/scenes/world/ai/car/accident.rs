//! Traffic accident controls. Spec: `docs/specs/ai-traffic.md` section 5.3.

use blackbox_driver::AiControls;

/// How a crash went for a traffic car. Only respawning clears the stopped state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Accident {
    None,
    InProgress(u32),
    Over,
}

/// The original counts thinks, not seconds.
const ACCIDENT_THINKS: u32 = 3;

impl Accident {
    /// A hit by the player's car starts the crash of a street car (a tractor stops at once). A hit while the car is
    /// in progress starts it again. A car that is over is started again only by a vehicle at 5 mph or faster (`fast`),
    /// the original's gate (`docs/specs/ai-traffic.md` §5.3 and `ai-traffic-obstacles.md` §5.3).
    pub fn on_hit(&mut self, traffic: bool, tractor: bool, fast: bool) {
        if !traffic || (*self == Self::Over && !fast) {
            return;
        }
        *self = match tractor {
            true => Self::Over,
            false => Self::InProgress(ACCIDENT_THINKS),
        };
    }

    pub fn think(&mut self) {
        let Self::InProgress(left) = *self else { return };
        *self = match left {
            0 | 1 => Self::Over,
            n => Self::InProgress(n - 1),
        };
    }

    /// The controls of a crash, or `None` when the car drives. A tractor writes no steering in either phase: it keeps the
    /// steering it had (`held_steer`), as the original does (`ai-traffic-obstacles.md` §5.2).
    pub fn controls(&self, tractor: bool, held_steer: f32) -> Option<AiControls> {
        let steer = match tractor {
            true => held_steer,
            false => 1.0,
        };
        match self {
            Self::None => None,
            Self::InProgress(_) => Some(AiControls { steer, ..AiControls::default() }),
            Self::Over => Some(AiControls { brake: 1.0, steer, ..AiControls::default() }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tractor_brakes_immediately_without_full_lock_even_after_losing_its_trailer() {
        let mut accident = Accident::None;
        accident.on_hit(true, true, false);
        assert_eq!(accident, Accident::Over);
        for _ in 0..10 {
            let controls = accident.controls(true, 0.0).unwrap();
            assert_eq!((controls.gas, controls.brake, controls.steer), (0.0, 1.0, 0.0));
            accident.think();
        }
    }

    #[test]
    fn cop_drivers_keep_driving_after_an_impact_with_the_player() {
        let mut accident = Accident::None;
        accident.on_hit(false, false, false);
        for _ in 0..10 {
            assert!(accident.controls(false, 0.0).is_none());
            accident.think();
        }
    }

    #[test]
    fn a_street_car_coasts_at_full_lock_for_three_thinks_then_stays_on_the_brake() {
        let mut accident = Accident::None;
        accident.on_hit(true, false, false);
        for _ in 0..3 {
            let controls = accident.controls(false, 0.0).unwrap();
            assert_eq!((controls.gas, controls.brake, controls.steer), (0.0, 0.0, 1.0));
            accident.think();
        }
        for _ in 0..10 {
            let controls = accident.controls(false, 0.0).unwrap();
            assert_eq!((controls.gas, controls.brake, controls.steer), (0.0, 1.0, 1.0));
            accident.think();
        }
    }

    #[test]
    fn a_hit_on_a_car_that_is_over_starts_it_again_only_from_a_moving_vehicle() {
        let mut accident = Accident::Over;
        accident.on_hit(true, false, false);
        assert_eq!(accident, Accident::Over, "a stopped hitter does not re-arm it");
        accident.on_hit(true, false, true);
        assert_eq!(accident, Accident::InProgress(ACCIDENT_THINKS), "a fast hitter does");
    }

    #[test]
    fn a_hit_while_in_progress_starts_the_three_thinks_again() {
        let mut accident = Accident::InProgress(1);
        accident.on_hit(true, false, false);
        assert_eq!(accident, Accident::InProgress(ACCIDENT_THINKS));
    }

    #[test]
    fn a_tractor_keeps_the_steering_it_had_in_both_phases() {
        let mut accident = Accident::None;
        accident.on_hit(true, true, false);
        assert_eq!(accident.controls(true, 0.4).unwrap().steer, 0.4);
        assert_eq!(accident.controls(true, -0.25).unwrap().steer, -0.25);
        accident = Accident::InProgress(ACCIDENT_THINKS);
        assert_eq!(accident.controls(true, 0.4).unwrap().steer, 0.4);
    }
}
