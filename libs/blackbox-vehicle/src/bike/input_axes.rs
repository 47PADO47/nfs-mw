//! The rider's input axes: the throttle, the brake levers and the stick do not jump to what the player holds, they move at the rates
//! of the vehicle's input data (`FInputData`: a rise and a fall rate per axis, per second; `docs/specs/ride6-feel-round8.md`, section 4).
//!
//! The constructor defaults of the reference are 10000 per second, which is "at once"; the bikes state 3 and 5 per second for the throttle
//! and the brake (a key takes a third of a second to open the throttle) and 25 and 6 for the stick. A rate of 0 here means "at once".

use super::BikeInput;

/// Rise and fall rates (per second) of the axes; 0 for an axis that follows its input at once.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputRates {
    /// `[rise, fall]` of the throttle, of the brake levers (the front and the rear lever alike) and of the stick. A rise is a move
    /// away from 0 (for the stick, in either direction); a fall is a move towards it.
    pub throttle: [f32; 2],
    pub brake: [f32; 2],
    pub steer: [f32; 2],
}

/// `value` moved towards `target` at most `rate x dt`, or at once when the rate is 0.
fn follow(value: f32, target: f32, rates: [f32; 2], dt: f32) -> f32 {
    let rising = target.abs() > value.abs() || target * value < 0.0;
    let rate = rates[usize::from(!rising)];
    if rate <= 0.0 {
        return target;
    }
    value + (target - value).clamp(-rate * dt, rate * dt)
}

impl InputRates {
    /// The axes after `dt` seconds of moving from `before` towards what the rider holds.
    pub fn apply(&self, before: &BikeInput, wanted: &BikeInput, dt: f32) -> BikeInput {
        // A stick that crosses the middle first falls to it, then rises on the other side: the rise rate alone would jump it.
        let steer = match before.steer * wanted.steer < 0.0 {
            true => follow(before.steer, 0.0, self.steer, dt),
            false => follow(before.steer, wanted.steer, self.steer, dt),
        };
        BikeInput {
            throttle: follow(before.throttle, wanted.throttle, self.throttle, dt),
            front_brake: follow(before.front_brake, wanted.front_brake, self.brake, dt),
            rear_brake: follow(before.rear_brake, wanted.rear_brake, self.brake, dt),
            steer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATES: InputRates = InputRates { throttle: [3.0, 5.0], brake: [3.0, 5.0], steer: [25.0, 6.0] };

    #[test]
    fn no_rates_follow_at_once() {
        let wanted = BikeInput { throttle: 1.0, front_brake: 1.0, rear_brake: 0.5, steer: -1.0 };
        assert_eq!(InputRates::default().apply(&BikeInput::default(), &wanted, 0.016), wanted);
    }

    #[test]
    fn a_key_takes_a_third_of_a_second_to_open_the_throttle_and_a_fifth_to_close_it() {
        let held = BikeInput { throttle: 1.0, ..Default::default() };
        let mut now = BikeInput::default();
        let mut time = 0.0_f32;
        while now.throttle < 1.0 {
            now = RATES.apply(&now, &held, 1.0 / 60.0);
            time += 1.0 / 60.0;
        }
        assert!((time - 1.0 / 3.0).abs() < 0.03, "{time}");
        let mut time = 0.0_f32;
        while now.throttle > 0.0 {
            now = RATES.apply(&now, &BikeInput::default(), 1.0 / 60.0);
            time += 1.0 / 60.0;
        }
        assert!((time - 0.2).abs() < 0.03, "{time}");
    }

    #[test]
    fn the_stick_goes_through_the_middle_when_it_is_thrown_across() {
        let left = BikeInput { steer: -0.5, ..Default::default() };
        let right = BikeInput { steer: 1.0, ..Default::default() };
        let next = RATES.apply(&left, &right, 0.02);
        assert!(next.steer > -0.5 && next.steer < 0.0, "{}", next.steer);
    }
}
