//! The infractions screen after a bust, the impound release and the game-over check
//! (`docs/specs/career-economy-flows.md` sections 6.1, 6.2 and 6.4).
//!
//! The marker inventory is the caller: [`InfractionsScreen::open`] says how many markers to grant, and the
//! marker choice ([`InfractionsScreen::use_marker`], [`release_with_marker`]) assumes the caller has removed the
//! marker from the inventory.

use super::infraction::{InfractionMask, fine_of_counters};
use crate::settings::{CareerSettings, Flags};
use crate::stable::impound::release_fee;
use crate::stable::{CareerRecord, ImpoundState, Stable};

/// Get Out of Jail Free markers granted the first time the screen is shown to a player who holds none. The
/// strings table promises three; the code adds four (`career-economy-flows.md` 6.1, open question 8). The code
/// wins.
pub const FIRST_VISIT_MARKERS: u8 = 4;

/// The outcome of paying the fines with cash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayOutcome {
    /// Paid in full and the car stays free; the screen closes.
    Paid,
    /// The strike limit was reached: the car is impounded (state 2, message `MW_BUSTED_STRIKE_LIMIT`).
    StrikeLimit,
    /// The cash did not cover the fine: the rest of the cash was confiscated and the car is impounded (state 3,
    /// message `MW_BUSTED_INSUFFICIENT_FUNDS`).
    InsufficientFunds,
}

/// The infractions screen of one bust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InfractionsScreen {
    /// The cash when the screen opened.
    pub cash_at_open: u32,
    /// The fine of this pursuit alone (each kind of its mask once).
    pub this_pursuit_fine: u32,
    /// The fine of every unserved infraction of the car.
    pub total_fine: u64,
    /// Markers to add to the inventory because this is the first visit (0 or [`FIRST_VISIT_MARKERS`]).
    pub markers_to_grant: u8,
}

impl InfractionsScreen {
    /// Opens the screen: sets the "infractions screen seen" flag, remembers the cash and computes both fines.
    /// `holds_jail_marker` is whether the player holds a Get Out of Jail Free marker (type `0x11`); the first
    /// visit grants [`FIRST_VISIT_MARKERS`] only to a player who holds none.
    pub fn open(
        settings: &mut CareerSettings,
        record: &CareerRecord,
        pursuit: InfractionMask,
        holds_jail_marker: bool,
    ) -> InfractionsScreen {
        let first_visit = !settings.flags.contains(Flags::INFRACTIONS_SEEN);
        settings.flags.insert(Flags::INFRACTIONS_SEEN);
        let markers_to_grant = if first_visit && !holds_jail_marker { FIRST_VISIT_MARKERS } else { 0 };
        InfractionsScreen {
            cash_at_open: settings.cash,
            this_pursuit_fine: pursuit.fine(),
            total_fine: fine_of_counters(&record.unserved),
            markers_to_grant,
        }
    }

    /// Pays every unserved infraction with cash, in the order of the spec:
    ///
    /// 1. `cash = 0` if the fine exceeds the cash, else `cash - fine`;
    /// 2. strikes + 1, decay counter 0;
    /// 3. all unserved infractions become served; heat 1.0;
    /// 4. the outcome: strike limit reached (`strikes >= limit`) wins over insufficient funds
    ///    (`fine > cash_at_open`); either impounds the car with `strikes = limit` and the countdown at 5.
    pub fn pay_with_cash(&self, settings: &mut CareerSettings, record: &mut CareerRecord) -> PayOutcome {
        let fine = fine_of_counters(&record.unserved);
        settings.cash = match fine > u64::from(settings.cash) {
            true => 0,
            false => settings.cash - fine as u32,
        };
        record.strikes = record.strikes.saturating_add(1);
        record.decay = 0;
        record.serve_all_unserved();
        record.heat = 1.0;
        if record.strikes >= record.strike_limit {
            record.enter_impound(ImpoundState::LimitReached);
            return PayOutcome::StrikeLimit;
        }
        if fine > u64::from(self.cash_at_open) {
            record.enter_impound(ImpoundState::CashConfiscated);
            return PayOutcome::InsufficientFunds;
        }
        PayOutcome::Paid
    }

    /// Uses a Get Out of Jail Free marker (the caller has removed it): this pursuit infraction kinds are taken
    /// out of the unserved set (one count each, not below 0) and the busted count drops by one. No strike, no
    /// cash change, heat unchanged; the other unserved infractions stay. The car is never impounded on this path.
    pub fn use_marker(&self, record: &mut CareerRecord, pursuit: InfractionMask) {
        for kind in pursuit.kinds() {
            let counter = &mut record.unserved[kind.index()];
            *counter = counter.saturating_sub(1);
        }
        record.busted = record.busted.saturating_sub(1);
    }
}

/// Why a release did not happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseError {
    /// The car is not in state 4 (free, or still counting down).
    NotReleasable,
    /// The cash does not cover the fee. The spec only says the fee is paid by `SpendCash`, which clamps; the
    /// release refuses instead of giving the car away for less (unconfirmed).
    InsufficientCash {
        /// The fee asked.
        fee: u32,
    },
}

/// Releases a car from the impound by paying `round(0.25 * cost)`. Returns the fee paid. The record is only
/// changed on success.
pub fn release_with_cash(
    settings: &mut CareerSettings,
    record: &mut CareerRecord,
    cost: u32,
) -> Result<u32, ReleaseError> {
    if !record.is_releasable() {
        return Err(ReleaseError::NotReleasable);
    }
    let fee = release_fee(cost);
    if !settings.can_afford(fee) {
        return Err(ReleaseError::InsufficientCash { fee });
    }
    settings.spend_cash(fee);
    record.release();
    Ok(fee)
}

/// Releases a car with an Impound Release marker (type `0x15`) instead of cash; the caller has removed the
/// marker. Returns false when the car is not in state 4 (the marker must then be kept).
pub fn release_with_marker(record: &mut CareerRecord) -> bool {
    record.release()
}

/// The game-over check of the car-select handler: when no live car is in state 0 and the player
/// holds no Impound Release marker, sets the game-over flag. Returns whether it is set. Cash is not consulted
/// (the strings hint that another check may exist, unconfirmed). The flag is cleared on every load.
pub fn apply_game_over_check(settings: &mut CareerSettings, stable: &Stable, holds_release_marker: bool) -> bool {
    if stable.free_car_count() >= 1 || holds_release_marker {
        return false;
    }
    settings.flags.insert(Flags::GAME_OVER);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::InfractionKind;
    use crate::stable::CarRecord;

    fn record_with(unserved: [u16; 8]) -> CareerRecord {
        CareerRecord { unserved, ..CareerRecord::new(0) }
    }

    fn settings_with_cash(cash: u32) -> CareerSettings {
        CareerSettings { cash, ..CareerSettings::new_career() }
    }

    #[test]
    fn opening_the_first_time_grants_four_markers_to_a_player_with_none() {
        let mut s = settings_with_cash(0);
        let r = record_with([0; 8]);
        let screen = InfractionsScreen::open(&mut s, &r, InfractionMask::NONE, false);
        assert_eq!(screen.markers_to_grant, 4);
        assert!(s.flags.contains(Flags::INFRACTIONS_SEEN));
        let again = InfractionsScreen::open(&mut s, &r, InfractionMask::NONE, false);
        assert_eq!(again.markers_to_grant, 0, "only the first visit");
    }

    #[test]
    fn a_player_holding_a_marker_is_not_given_more() {
        let mut s = settings_with_cash(0);
        let screen = InfractionsScreen::open(&mut s, &record_with([0; 8]), InfractionMask::NONE, true);
        assert_eq!(screen.markers_to_grant, 0);
        assert!(s.flags.contains(Flags::INFRACTIONS_SEEN), "the flag is set anyway");
    }

    #[test]
    fn the_screen_computes_both_fines() {
        let mut s = settings_with_cash(777);
        let r = record_with([2, 0, 1, 0, 0, 0, 0, 0]);
        let pursuit = InfractionMask::of(&[InfractionKind::Speeding, InfractionKind::RecklessDriving]);
        let screen = InfractionsScreen::open(&mut s, &r, pursuit, false);
        assert_eq!(screen.cash_at_open, 777);
        assert_eq!(screen.this_pursuit_fine, 1150);
        assert_eq!(screen.total_fine, 2 * 150 + 1000);
    }

    #[test]
    fn paying_with_enough_cash_takes_a_strike_and_serves_the_infractions() {
        let mut s = settings_with_cash(5000);
        let mut r = record_with([1, 1, 0, 0, 0, 0, 0, 0]);
        r.decay = 2;
        let screen = InfractionsScreen::open(&mut s, &r, InfractionMask::NONE, true);
        assert_eq!(screen.pay_with_cash(&mut s, &mut r), PayOutcome::Paid);
        assert_eq!(s.cash, 5000 - 500);
        assert_eq!((r.strikes, r.decay, r.heat, r.state), (1, 0, 1.0, 0));
        assert_eq!((r.unserved_total(), r.served), (0, [1, 1, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn paying_exactly_the_cash_leaves_zero_and_is_not_a_confiscation() {
        let mut s = settings_with_cash(500);
        let mut r = record_with([1, 1, 0, 0, 0, 0, 0, 0]);
        let screen = InfractionsScreen::open(&mut s, &r, InfractionMask::NONE, true);
        assert_eq!(screen.pay_with_cash(&mut s, &mut r), PayOutcome::Paid);
        assert_eq!(s.cash, 0);
        assert_eq!(r.state, 0);
    }

    #[test]
    fn a_fine_above_the_cash_confiscates_it_and_impounds() {
        let mut s = settings_with_cash(499);
        let mut r = record_with([1, 1, 0, 0, 0, 0, 0, 0]);
        let screen = InfractionsScreen::open(&mut s, &r, InfractionMask::NONE, true);
        assert_eq!(screen.pay_with_cash(&mut s, &mut r), PayOutcome::InsufficientFunds);
        assert_eq!(s.cash, 0);
        assert_eq!((r.state, r.strikes, r.countdown), (3, 3, 5), "strikes jump to the limit");
        assert_eq!(r.unserved_total(), 0, "the infractions still move to served");
    }

    #[test]
    fn the_third_strike_impounds_even_when_the_cash_covers_the_fine() {
        let mut s = settings_with_cash(10_000);
        let mut r = record_with([1, 0, 0, 0, 0, 0, 0, 0]);
        r.strikes = 2;
        let screen = InfractionsScreen::open(&mut s, &r, InfractionMask::NONE, true);
        assert_eq!(screen.pay_with_cash(&mut s, &mut r), PayOutcome::StrikeLimit);
        assert_eq!(s.cash, 10_000 - 150);
        assert_eq!((r.state, r.strikes, r.countdown), (2, 3, 5));
    }

    #[test]
    fn the_strike_limit_wins_when_both_apply() {
        let mut s = settings_with_cash(100);
        let mut r = record_with([1, 0, 0, 0, 0, 0, 0, 0]);
        r.strikes = 2;
        let screen = InfractionsScreen::open(&mut s, &r, InfractionMask::NONE, true);
        assert_eq!(screen.pay_with_cash(&mut s, &mut r), PayOutcome::StrikeLimit);
        assert_eq!(r.state, 2);
        assert_eq!(s.cash, 0);
    }

    #[test]
    fn a_raised_limit_delays_the_impound() {
        let mut s = settings_with_cash(10_000);
        let mut r = record_with([1, 0, 0, 0, 0, 0, 0, 0]);
        r.strikes = 2;
        r.strike_limit = 5;
        let screen = InfractionsScreen::open(&mut s, &r, InfractionMask::NONE, true);
        assert_eq!(screen.pay_with_cash(&mut s, &mut r), PayOutcome::Paid);
        assert_eq!(r.strikes, 3);
    }

    #[test]
    fn an_empty_fine_still_costs_a_strike() {
        let mut s = settings_with_cash(10);
        let mut r = record_with([0; 8]);
        let screen = InfractionsScreen::open(&mut s, &r, InfractionMask::NONE, true);
        assert_eq!(screen.pay_with_cash(&mut s, &mut r), PayOutcome::Paid);
        assert_eq!((s.cash, r.strikes), (10, 1));
    }

    #[test]
    fn a_marker_removes_this_pursuit_only_and_costs_nothing() {
        let mut s = settings_with_cash(300);
        let mut r = record_with([3, 1, 0, 0, 2, 0, 0, 0]);
        r.busted = 2;
        r.heat = 4.0;
        let pursuit =
            InfractionMask::of(&[InfractionKind::Speeding, InfractionKind::HitAndRun, InfractionKind::OffRoad]);
        let screen = InfractionsScreen::open(&mut s, &r, pursuit, true);
        screen.use_marker(&mut r, pursuit);
        assert_eq!(r.unserved, [2, 1, 0, 0, 1, 0, 0, 0], "off road was already 0 and stays 0");
        assert_eq!((r.busted, r.strikes, r.heat, r.state), (1, 0, 4.0, 0));
        assert_eq!(s.cash, 300);
    }

    #[test]
    fn the_busted_count_never_goes_below_zero() {
        let mut r = record_with([0; 8]);
        let mut s = settings_with_cash(0);
        let screen = InfractionsScreen::open(&mut s, &r, InfractionMask::NONE, true);
        screen.use_marker(&mut r, InfractionMask::NONE);
        assert_eq!(r.busted, 0);
    }

    fn releasable() -> CareerRecord {
        let mut r = CareerRecord::new(0);
        r.enter_impound(ImpoundState::LimitReached);
        r.countdown = 1;
        r.win_tick();
        r
    }

    #[test]
    fn a_cash_release_pays_a_quarter_of_the_cost() {
        let mut s = settings_with_cash(10_000);
        let mut r = releasable();
        r.heat = 3.0;
        assert_eq!(release_with_cash(&mut s, &mut r, 35_000), Ok(8_750));
        assert_eq!(s.cash, 1_250);
        assert_eq!((r.state, r.strikes, r.countdown, r.heat), (0, 0, 0, 0.0));
    }

    #[test]
    fn a_release_needs_state_four_and_enough_cash() {
        let mut s = settings_with_cash(8_749);
        let mut r = releasable();
        assert_eq!(release_with_cash(&mut s, &mut r, 35_000), Err(ReleaseError::InsufficientCash { fee: 8_750 }));
        assert_eq!((s.cash, r.state), (8_749, 4));
        let mut counting = CareerRecord::new(1);
        counting.enter_impound(ImpoundState::CashConfiscated);
        assert_eq!(release_with_cash(&mut s, &mut counting, 1), Err(ReleaseError::NotReleasable));
        assert!(!release_with_marker(&mut counting));
        assert_eq!(counting.state, 3);
        assert!(release_with_marker(&mut r));
        assert_eq!(r.state, 0);
    }

    #[test]
    fn game_over_needs_no_free_car_and_no_release_marker() {
        let mut stable = Stable::empty();
        stable.cars[0] = CarRecord { handle: 1, flags: 0xF_0002, career_index: 0, ..CarRecord::free() };
        let mut impounded = CareerRecord::new(0);
        impounded.enter_impound(ImpoundState::LimitReached);
        stable.careers[0] = impounded;
        let mut s = CareerSettings::new_career();
        assert!(!apply_game_over_check(&mut s, &stable, true), "a release marker saves the player");
        assert!(!s.flags.contains(Flags::GAME_OVER));
        assert!(apply_game_over_check(&mut s, &stable, false));
        assert!(s.flags.contains(Flags::GAME_OVER));
        stable.careers[0] = CareerRecord::new(0);
        let mut fresh = CareerSettings::new_career();
        assert!(!apply_game_over_check(&mut fresh, &stable, false));
    }
}
