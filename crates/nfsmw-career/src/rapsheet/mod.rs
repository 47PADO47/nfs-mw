//! The rap sheet: the unlock gate, the screen graph, the numbers on every page, the rankings, the pursuit history
//! and the profile card percentages (`docs/specs/career-rapsheet.md`, `docs/specs/career-rapsheet-2.md`, with the
//! corrected tables of `career-economy-flows.md` and the verdict rule of `career-text.md`).
//!
//! Pure rules over the stable and the history: no text (labels are keys, the strings come from the language
//! table) and no screens.
//!
//! - [`nav`]: the entry gate, pages and Back targets, the login pair, the remembered selection, the end-game chain.
//! - [`session`]: the remembered selection table, the ranking views and the end-game chain.
//! - [`pages`]: totals, the summary verdict, the infractions, vehicle, cost to state and top 5 rows.
//! - [`rank`]: the rank rule, the threshold tables and the ranking detail rows.
//! - [`history`]: the history block and how a pursuit is recorded (top 5 by bounty, best pairs, sums).
//! - [`card`]: the completion percentages of the safe house menu and the main menu card.
//! - [`time`]: the pursuit length formatter.

pub mod card;
pub mod history;
pub mod nav;
pub mod pages;
pub mod rank;
pub mod session;
pub mod time;

pub use card::{CardCounts, ProfileCard, SeriesRecord, count_series, lifetime_ranks, rap_percent};
pub use history::{
    BestPair, HISTORY_LEN, PursuitEntry, PursuitHistory, PursuitStats, Recorded, TOP_ENTRIES, case_id_text,
};
pub use nav::{
    LoginMode, LoginPage, Page, back_target, enter_rap_sheet, leave_to_safe_house, main_button_page,
    ranking_category_page, rap_sheet_unlocked, top_entry_page,
};
pub use pages::{
    CostRow, DriveStatus, InfractionRow, InfractionView, RapTotals, SUMMARY_THRESHOLDS, Step, VehicleRow, cost_rows,
    infraction_rows, statistic, summary_index, summary_label, top_step, top_used, vehicle_rows,
};
pub use rank::{RankingRow, UNRANKED, rank, rank_default, ranking_rows};
pub use session::{EndGame, EndGamePage, RankingViews, Remembered, SelectionMemory, highlight_button};
pub use time::format_ticks;
