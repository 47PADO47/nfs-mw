//! Which cars a support request takes, and which model each one is (`AcquireHeavyCars` and `AcquireCrossCars`, spec
//! `ai-pursuit-support.md` sections 4 and 5). The game maps each [`Car`] to its car model; the names stay out of here.

use super::Strategy;

/// A support car by its role. `BigSuv` and `Suv` are the two heavy models; `Cross` is the leader car and `Hench` the two
/// cars that fly the V with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Car {
    BigSuv,
    Suv,
    Cross,
    Hench,
}

/// How many cars a strategy takes (the acquisition counts of the spec, sections 4 and 5).
pub fn cars_for(strategy: Strategy) -> u32 {
    match strategy {
        Strategy::EBrake | Strategy::CrossFollow | Strategy::CrossBrake => 1,
        Strategy::CoordinatedEBrake | Strategy::Ram => 2,
        Strategy::CrossPlusVBlock => 3,
        Strategy::HeavyRoadblock => 4,
    }
}

/// The car the request takes as its `index`-th car, with `rand` uniform in `0..=1` for the SUV roll. A heavy strategy takes
/// SUVs, the big one with `big_suv` percent; a leader strategy takes the cross car first and then the two V cars.
pub fn car_for(strategy: Strategy, index: u32, big_suv: f32, rand: f32) -> Car {
    match strategy.is_heavy() {
        true => suv(big_suv, rand),
        false => leader(index),
    }
}

/// The SUV model: `copsuv` (here [`Car::BigSuv`]) with `big_suv` percent, otherwise the smaller one. The binary's roll
/// is `0..99` and the car is big when the roll is under the chance (spec section 4).
pub fn suv(big_suv: f32, rand: f32) -> Car {
    match percent(rand) < big_suv {
        true => Car::BigSuv,
        false => Car::Suv,
    }
}

/// The leader car at `index`: the first is the cross car, the rest are the V cars.
pub fn leader(index: u32) -> Car {
    match index {
        0 => Car::Cross,
        _ => Car::Hench,
    }
}

/// A uniform `0..=1` number as the binary's whole percentage `0..=99`.
fn percent(rand: f32) -> f32 {
    (rand.clamp(0.0, 1.0) * 100.0).floor().min(99.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_heavy_strategies_take_the_spec_counts() {
        assert_eq!(cars_for(Strategy::EBrake), 1);
        assert_eq!(cars_for(Strategy::CoordinatedEBrake), 2);
        assert_eq!(cars_for(Strategy::Ram), 2);
        assert_eq!(cars_for(Strategy::HeavyRoadblock), 4);
    }

    #[test]
    fn the_leader_strategies_take_the_spec_counts() {
        assert_eq!(cars_for(Strategy::CrossFollow), 1);
        assert_eq!(cars_for(Strategy::CrossBrake), 1);
        assert_eq!(cars_for(Strategy::CrossPlusVBlock), 3);
    }

    #[test]
    fn the_suv_roll_is_big_under_the_chance() {
        assert_eq!(suv(50.0, 0.49), Car::BigSuv, "roll 49 is under 50");
        assert_eq!(suv(50.0, 0.50), Car::Suv, "roll 50 is not under 50");
        assert_eq!(suv(100.0, 0.999), Car::BigSuv, "a 100 percent chance always gives the big SUV");
        assert_eq!(suv(0.0, 0.0), Car::Suv, "a 0 percent chance never does");
    }

    #[test]
    fn the_roll_stays_in_zero_to_ninety_nine() {
        assert_eq!(percent(1.0), 99.0);
        assert_eq!(percent(0.0), 0.0);
        assert_eq!(percent(-0.5), 0.0, "a bad input is clamped");
    }

    #[test]
    fn a_heavy_request_asks_the_roll_and_a_leader_request_the_leader_cars() {
        assert_eq!(car_for(Strategy::Ram, 1, 100.0, 0.3), Car::BigSuv);
        assert_eq!(car_for(Strategy::CrossPlusVBlock, 0, 0.0, 0.3), Car::Cross);
        assert_eq!(car_for(Strategy::CrossPlusVBlock, 1, 0.0, 0.3), Car::Hench);
        assert_eq!(car_for(Strategy::CrossPlusVBlock, 2, 0.0, 0.3), Car::Hench);
    }
}
