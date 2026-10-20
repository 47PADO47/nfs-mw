//! The career economy rules as pure functions over the settings and the stable
//! (`docs/specs/career-economy.md`, `docs/specs/career-economy-flows.md`).
//!
//! - [`infraction`]: the eight kinds and the fine table.
//! - [`pursuit`]: what a finished player pursuit writes into the car record (flows section 2).
//! - [`infractions_screen`]: the bust screen: pay with cash or a marker, the strike limit, the release (6.1, 6.2).
//! - [`aftermath`]: the notices and the car select after the infractions screen (`busted-aftermath.md`).
//! - [`cooling`]: the paint cooling factors and the heat meter (`career-cooling.md`).
//! - [`cash`]: race-win cash, sale proceeds and the bounty award hook (sections 3.4 and 7).
//! - [`markers`]: the held markers (Get Out of Jail Free, Extra Impound Strike, Impound Release) in their table (6.5).
//!
//! Left out on purpose: the profile pursuit history and rankings (flows section 5) and the heat factors (they need
//! the `pursuitlevels` data).

pub mod aftermath;
pub mod cash;
pub mod cooling;
pub mod infraction;
pub mod infractions_screen;
pub mod markers;
pub mod pursuit;

pub use cash::{AWARD_BOUNTY_DIVISOR, RaceWin, award_player_bounty, award_race_win, race_win_cash, sale_proceeds};
pub use infraction::{FINES, InfractionKind, InfractionMask, fine_of_counters};
pub use infractions_screen::{
    FIRST_VISIT_MARKERS, InfractionsScreen, PayOutcome, ReleaseError, apply_game_over_check, release_with_cash,
    release_with_marker,
};
pub use pursuit::{PursuitEnd, PursuitResult, RecordedPursuit, end_player_pursuit, record_pursuit};
