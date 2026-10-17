//! The impound strike rules on one career record (`docs/specs/career-economy-flows.md` sections 6.1 to 6.3 and
//! 6.5): strikes and their limit, entering and leaving the impound, the two tickers.
//!
//! The infractions screen that drives these is in [`crate::economy`]; this file holds only the record-level
//! transitions so each can be tested alone.

use super::career_record::{CareerRecord, ImpoundState};
use crate::economy::InfractionMask;

/// The most the strike limit can be raised to by Extra Impound Strike markers.
pub const MAX_STRIKE_LIMIT: i8 = 5;
/// The countdown, in events, of a car that has just been impounded.
pub const IMPOUND_COUNTDOWN: u8 = 5;
/// Evaded pursuits (while free) it takes to remove one strike: the decay counter lowers a strike when it
/// exceeds 2, so on the third.
pub const EVADES_PER_STRIKE_DECAY: u8 = 3;
/// The release fee as a share of the car `Cost`: `round(0.25 * Cost)`.
pub const RELEASE_FEE_DIVISOR: u32 = 4;

/// The release fee of a car of price `cost`: `round(0.25 * cost)`. Halves round up (away from zero), which is the
/// C `round` reading of the spec; the original rounding mode for exact halves is not stated (unconfirmed).
pub fn release_fee(cost: u32) -> u32 {
    ((u64::from(cost) + u64::from(RELEASE_FEE_DIVISOR / 2)) / u64::from(RELEASE_FEE_DIVISOR)) as u32
}

impl CareerRecord {
    /// Adds one count to the unserved counter of every kind in `mask`, saturating at `u16::MAX`.
    pub fn add_unserved(&mut self, mask: InfractionMask) {
        for kind in mask.kinds() {
            let counter = &mut self.unserved[kind.index()];
            *counter = counter.saturating_add(1);
        }
    }

    /// Moves every unserved infraction into the served counters.
    pub fn serve_all_unserved(&mut self) {
        for (served, unserved) in self.served.iter_mut().zip(self.unserved.iter_mut()) {
            *served = served.saturating_add(*unserved);
            *unserved = 0;
        }
    }

    /// Puts the car in an impound state (2 or 3): `strikes = limit` and the countdown starts at 5.
    pub fn enter_impound(&mut self, state: ImpoundState) {
        self.set_impound_state(state);
        self.strikes = self.strike_limit;
        self.countdown = IMPOUND_COUNTDOWN;
    }

    /// The tick that runs when the player evades a pursuit.
    ///
    /// A free car counts the evade: when the decay counter exceeds 2 a strike is removed and the counter
    /// restarts; strikes never end below 0. An impounded car counts down instead. Returns true when this tick
    /// made the car releasable.
    pub fn evade_tick(&mut self) -> bool {
        if !self.is_free_to_drive() {
            return self.countdown_tick();
        }
        self.decay = self.decay.saturating_add(1);
        if self.decay >= EVADES_PER_STRIKE_DECAY {
            self.strikes = self.strikes.saturating_sub(1);
            self.decay = 0;
        }
        self.strikes = self.strikes.max(0);
        false
    }

    /// The tick that runs when the player wins a career race: only impounded cars react. Returns
    /// true when this tick made the car releasable (the caller then sends the "released" message, SMS 120).
    pub fn win_tick(&mut self) -> bool {
        if self.is_free_to_drive() {
            return false;
        }
        self.countdown_tick()
    }

    /// One countdown step: lower the countdown if above 0, and when it is 0 and the state is not yet 4, make the
    /// car releasable. The specs read as "five ticks after the impound, then release": the state changes on the
    /// tick that brings the countdown to 0 (the other reading, one tick later, is unconfirmed).
    fn countdown_tick(&mut self) -> bool {
        self.countdown = self.countdown.saturating_sub(1);
        if self.countdown != 0 || self.impound_state() == ImpoundState::Releasable {
            return false;
        }
        self.set_impound_state(ImpoundState::Releasable);
        true
    }

    /// Applies an Extra Impound Strike marker (type `0x14`): raises the limit by 1, to at most 5, for a free car
    /// whose limit is below 5. Returns false when the marker does not apply and must not be consumed.
    pub fn apply_extra_strike_marker(&mut self) -> bool {
        if !self.is_free_to_drive() || self.strike_limit >= MAX_STRIKE_LIMIT {
            return false;
        }
        self.strike_limit += 1;
        true
    }

    /// True when the car waits for a release fee or an Impound Release marker (state 4).
    pub fn is_releasable(&self) -> bool {
        self.impound_state() == ImpoundState::Releasable
    }

    /// Frees a releasable car (by fee or by marker, which the caller has paid or consumed): state 0, strikes 0,
    /// countdown 0, heat 0. Returns false and changes nothing when the car is not in state 4.
    pub fn release(&mut self) -> bool {
        if !self.is_releasable() {
            return false;
        }
        self.set_impound_state(ImpoundState::Free);
        self.strikes = 0;
        self.countdown = 0;
        self.heat = 0.0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::InfractionKind;

    #[test]
    fn the_release_fee_is_a_quarter_of_the_cost() {
        assert_eq!(release_fee(35_000), 8_750);
        assert_eq!(release_fee(150_000), 37_500);
        assert_eq!(release_fee(0), 0);
        assert_eq!(release_fee(10_001), 2_500);
        assert_eq!(release_fee(10_002), 2_501, "an exact half rounds up");
        assert_eq!(release_fee(10_003), 2_501);
        assert_eq!(release_fee(u32::MAX), 1_073_741_824);
    }

    #[test]
    fn infractions_are_added_one_per_kind() {
        let mut r = CareerRecord::new(0);
        let mask = InfractionMask::of(&[InfractionKind::Racing, InfractionKind::OffRoad]);
        r.add_unserved(mask);
        r.add_unserved(mask);
        assert_eq!(r.unserved, [0, 2, 0, 0, 0, 0, 0, 2]);
        r.unserved[1] = u16::MAX;
        r.add_unserved(mask);
        assert_eq!(r.unserved[1], u16::MAX);
    }

    #[test]
    fn serving_moves_the_unserved_counters() {
        let mut r = CareerRecord::new(0);
        r.unserved = [1, 2, 3, 0, 0, 0, 0, 5];
        r.served[1] = 10;
        r.serve_all_unserved();
        assert_eq!(r.unserved, [0; 8]);
        assert_eq!(r.served, [1, 12, 3, 0, 0, 0, 0, 5]);
    }

    #[test]
    fn entering_the_impound_sets_strikes_to_the_limit_and_starts_the_countdown() {
        let mut r = CareerRecord::new(0);
        r.strikes = 1;
        r.enter_impound(ImpoundState::CashConfiscated);
        assert_eq!((r.state, r.strikes, r.countdown), (3, 3, 5));
    }

    #[test]
    fn every_third_evade_removes_a_strike() {
        let mut r = CareerRecord::new(0);
        r.strikes = 2;
        assert!(!r.evade_tick());
        assert!(!r.evade_tick());
        assert_eq!((r.strikes, r.decay), (2, 2));
        r.evade_tick();
        assert_eq!((r.strikes, r.decay), (1, 0));
        for _ in 0..3 {
            r.evade_tick();
        }
        assert_eq!(r.strikes, 0);
        for _ in 0..3 {
            r.evade_tick();
        }
        assert_eq!(r.strikes, 0, "strikes never go below 0");
    }

    #[test]
    fn a_negative_strike_count_is_lifted_to_zero_on_an_evade() {
        let mut r = CareerRecord::new(0);
        r.strikes = -1;
        r.evade_tick();
        assert_eq!(r.strikes, 0);
    }

    #[test]
    fn five_ticks_release_an_impounded_car_and_win_ticks_count_like_evades() {
        let mut r = CareerRecord::new(0);
        r.enter_impound(ImpoundState::LimitReached);
        assert!(!r.evade_tick()); // 4
        assert!(!r.win_tick()); // 3
        assert!(!r.evade_tick()); // 2
        assert!(!r.win_tick()); // 1
        assert!(r.win_tick(), "the fifth tick makes the car releasable");
        assert_eq!((r.state, r.countdown), (4, 0));
        assert!(!r.win_tick(), "no second release message");
        assert!(!r.evade_tick());
        assert_eq!(r.state, 4);
    }

    #[test]
    fn the_win_tick_ignores_free_cars_and_the_evade_tick_does_not_decay_impounded_ones() {
        let mut free = CareerRecord::new(0);
        free.strikes = 2;
        free.decay = 2;
        assert!(!free.win_tick());
        assert_eq!((free.strikes, free.decay), (2, 2));
        let mut impounded = CareerRecord::new(1);
        impounded.enter_impound(ImpoundState::LimitReached);
        impounded.evade_tick();
        assert_eq!((impounded.strikes, impounded.decay, impounded.countdown), (3, 0, 4));
    }

    #[test]
    fn the_extra_strike_marker_raises_the_limit_to_five() {
        let mut r = CareerRecord::new(0);
        assert!(r.apply_extra_strike_marker());
        assert!(r.apply_extra_strike_marker());
        assert_eq!(r.strike_limit, 5);
        assert!(!r.apply_extra_strike_marker(), "the cap is 5");
        let mut impounded = CareerRecord::new(1);
        impounded.enter_impound(ImpoundState::LimitReached);
        assert!(!impounded.apply_extra_strike_marker(), "only a free car");
        assert_eq!(impounded.strike_limit, 3);
    }

    #[test]
    fn release_needs_state_four_and_resets_the_car() {
        let mut r = CareerRecord::new(0);
        r.enter_impound(ImpoundState::LimitReached);
        r.heat = 2.5;
        assert!(!r.release(), "still counting down");
        assert_eq!(r.state, 2);
        for _ in 0..5 {
            r.win_tick();
        }
        assert!(r.release());
        assert_eq!((r.state, r.strikes, r.countdown, r.heat), (0, 0, 0, 0.0));
        assert_eq!(r.strike_limit, 3, "the limit survives a release");
        assert!(!r.release(), "a free car cannot be released");
    }
}
