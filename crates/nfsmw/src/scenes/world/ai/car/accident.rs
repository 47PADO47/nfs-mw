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
    pub fn on_hit(&mut self, traffic: bool, tractor: bool) {
        if !traffic || *self != Self::None {
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

    pub fn controls(&self, tractor: bool) -> Option<AiControls> {
        let steer = match tractor {
            true => 0.0,
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
        accident.on_hit(true, true);
        assert_eq!(accident, Accident::Over);
        for _ in 0..10 {
            let controls = accident.controls(true).unwrap();
            assert_eq!((controls.gas, controls.brake, controls.steer), (0.0, 1.0, 0.0));
            accident.think();
        }
    }

    #[test]
    fn cop_drivers_keep_driving_after_an_impact_with_the_player() {
        let mut accident = Accident::None;
        accident.on_hit(false, false);
        for _ in 0..10 {
            assert!(accident.controls(false).is_none());
            accident.think();
        }
    }

    #[test]
    fn a_street_car_coasts_at_full_lock_for_three_thinks_then_stays_on_the_brake() {
        let mut accident = Accident::None;
        accident.on_hit(true, false);
        for _ in 0..3 {
            let controls = accident.controls(false).unwrap();
            assert_eq!((controls.gas, controls.brake, controls.steer), (0.0, 0.0, 1.0));
            accident.think();
        }
        for _ in 0..10 {
            let controls = accident.controls(false).unwrap();
            assert_eq!((controls.gas, controls.brake, controls.steer), (0.0, 1.0, 1.0));
            accident.think();
        }
    }
}
