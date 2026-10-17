//! Cash and bounty flows other than fines (`docs/specs/career-economy-flows.md` sections 3.4 and 7).

use crate::settings::CareerSettings;
use crate::stable::{CareerRecord, Stable};

/// The divisor `AwardPlayerBounty` applies to the running bounty after adding the award.
///
/// The file image holds 5, but no instruction writes the value and dividing every award by 5 would erase the
/// bounty at each speed trap, so the spec says to treat it as **1** (`career-economy-flows.md` 3.4,
/// **[unconfirmed]**: it needs a run-time read of the original). With 1 the hook is `bounty += amount`.
pub const AWARD_BOUNTY_DIVISOR: u32 = 1;

/// `AwardPlayerBounty(amount)`: `bounty = (bounty + amount) / D` with `D` = [`AWARD_BOUNTY_DIVISOR`], on the
/// record of the current car. The sum saturates at `u32::MAX`. Returns the new bounty.
pub fn award_player_bounty(record: &mut CareerRecord, amount: u32) -> u32 {
    record.bounty = record.bounty.saturating_add(amount) / AWARD_BOUNTY_DIVISOR;
    record.bounty
}

/// The cash a career race win pays: `round(CashValue)`, halves away from zero. A negative or non-number value
/// pays 0, and a value past `u32::MAX` saturates.
pub fn race_win_cash(cash_value: f32) -> u32 {
    if cash_value.is_nan() || cash_value <= 0.0 {
        return 0;
    }
    cash_value.round() as u32
}

/// What a career race win did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaceWin {
    /// The cash added.
    pub cash: u32,
    /// How many impounded cars became releasable; the caller sends SMS 120 when this is not 0.
    pub released_cars: usize,
}

/// A career race win in race mode 2, placed first: pays the event `CashValue` on **every** win (not only the
/// first) and runs the impound countdown tick over the stable. The first-completion rewards and the heat factor
/// belong to the callers.
pub fn award_race_win(settings: &mut CareerSettings, stable: &mut Stable, cash_value: f32) -> RaceWin {
    let cash = race_win_cash(cash_value);
    settings.add_cash(cash);
    RaceWin { cash, released_cars: stable.win_tick() }
}

/// What selling a car pays: half the `Cost`, an integer shift. The sold car numbers move into the history with
/// [`crate::stable::SoldHistory::absorb`] first.
pub const fn sale_proceeds(cost: u32) -> u32 {
    cost >> 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stable::{CarRecord, ImpoundState};

    #[test]
    fn the_bounty_hook_adds_with_a_divisor_of_one() {
        assert_eq!(AWARD_BOUNTY_DIVISOR, 1);
        let mut r = CareerRecord::new(0);
        assert_eq!(award_player_bounty(&mut r, 4000), 4000);
        assert_eq!(award_player_bounty(&mut r, 8000), 12_000);
        assert_eq!(r.bounty, 12_000);
        r.bounty = u32::MAX - 1;
        assert_eq!(award_player_bounty(&mut r, 5), u32::MAX);
    }

    #[test]
    fn race_cash_rounds_and_clamps() {
        assert_eq!(race_win_cash(25_000.0), 25_000);
        assert_eq!(race_win_cash(1499.5), 1500);
        assert_eq!(race_win_cash(1499.4), 1499);
        assert_eq!(race_win_cash(0.0), 0);
        assert_eq!(race_win_cash(-5.0), 0);
        assert_eq!(race_win_cash(f32::NAN), 0);
        assert_eq!(race_win_cash(1.0e12), u32::MAX);
    }

    #[test]
    fn a_win_pays_every_time_and_ticks_the_impound() {
        let mut settings = CareerSettings::new_career();
        let mut stable = Stable::empty();
        stable.cars[0] = CarRecord { handle: 5, flags: 0xF_0002, career_index: 0, ..CarRecord::free() };
        let mut impounded = CareerRecord::new(0);
        impounded.enter_impound(ImpoundState::LimitReached);
        impounded.countdown = 1;
        stable.careers[0] = impounded;
        let first = award_race_win(&mut settings, &mut stable, 1500.0);
        assert_eq!((first.cash, first.released_cars), (1500, 1));
        let second = award_race_win(&mut settings, &mut stable, 1500.0);
        assert_eq!((second.cash, second.released_cars), (1500, 0));
        assert_eq!(settings.cash, 3000);
    }

    #[test]
    fn selling_pays_half_the_cost_with_an_integer_shift() {
        assert_eq!(sale_proceeds(35_000), 17_500);
        assert_eq!(sale_proceeds(35_001), 17_500);
        assert_eq!(sale_proceeds(1), 0);
    }
}
