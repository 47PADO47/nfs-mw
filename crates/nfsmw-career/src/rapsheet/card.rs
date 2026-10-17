//! The profile card and the completion percentages (`docs/specs/career-rapsheet-2.md` section 4).
//!
//! The safe house menu prints `career` as `CAREER_DATA`; the main menu card prints all four bytes. The counts of
//! milestones and events come from the gameplay data and the race records, so they are inputs.

use super::history::PursuitHistory;
use super::rank::{UNRANKED, rank_default};
use crate::stable::Stable;

/// What the card needs from the milestone table and the race records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CardCounts {
    /// Milestones with state 4 (done).
    pub milestones_done: u32,
    /// Milestone count of the manager (55 in the install).
    pub milestones_total: u32,
    /// Events of bins 1 to 16 (lists A and B of each bin view) whose race record has flag 2 (completed in career).
    pub events_done: u32,
    /// All events of bins 1 to 16.
    pub events_total: u32,
    /// Challenge series records flagged completed outside career (flag 1).
    pub series_done: u32,
    /// Challenge series records counted (see [`SeriesRecord`]).
    pub series_total: u32,
}

/// A race record as the challenge series count sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeriesRecord {
    /// The record is named `19.8.31`: skipped.
    pub is_event_19_8_31: bool,
    /// The definition flags have bit 15 (challenge series).
    pub is_challenge_series: bool,
    /// The record flag 1: completed outside career.
    pub completed_outside_career: bool,
}

/// Counts the series records: every record that is not `19.8.31`, is flagged challenge series and passes the debug
/// gate (the debug-event byte is clear, or the developer flag is set) counts in the total, and in the done count
/// when it was completed outside career.
pub fn count_series(records: &[SeriesRecord], debug_event_byte: bool, developer_flag: bool) -> (u32, u32) {
    let gate_open = !debug_event_byte || developer_flag;
    let mut done = 0;
    let mut total = 0;
    for r in records {
        if r.is_event_19_8_31 || !r.is_challenge_series || !gate_open {
            continue;
        }
        total += 1;
        if r.completed_outside_career {
            done += 1;
        }
    }
    (done, total)
}

/// The four bytes of the card and the series pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileCard {
    /// Overall completion, percent.
    pub game: u8,
    /// Career completion, percent (`CAREER_DATA`).
    pub career: u8,
    /// Rap sheet completion, percent.
    pub rap: u8,
    /// Challenge series completion, percent.
    pub series: u8,
    /// Series records completed.
    pub series_done: u32,
    /// Series records counted.
    pub series_total: u32,
}

fn percent(numerator: u64, denominator: u64) -> u8 {
    if denominator == 0 {
        return 0;
    }
    (100 * numerator / denominator) as u8
}

/// The rap sheet percentage: `trunc(100 * sum over the ten statistics of max(0, 15 - rank) / 140)`, with the
/// lifetime ranks (a rank of 16 adds nothing).
pub fn rap_percent(ranks: &[u8; 10]) -> u8 {
    let points: u64 = ranks.iter().map(|&r| u64::from(15u8.saturating_sub(r))).sum();
    percent(points, 140)
}

/// The lifetime rank of each of the ten statistics from the stable and the history, with the default tables.
pub fn lifetime_ranks(stable: &Stable, history: &PursuitHistory) -> [u8; 10] {
    let mut ranks = [UNRANKED; 10];
    for (stat, rank) in ranks.iter_mut().enumerate() {
        if let Some(value) = super::pages::statistic(stable, history, stat) {
            *rank = rank_default(stat, true, value);
        }
    }
    ranks
}

impl ProfileCard {
    /// The card: `career = trunc(100 * (ms_done + ev_done) / (ms_total + ev_total))`, `rap` from the lifetime
    /// ranks, `series = trunc(100 * done / total)` and `game = trunc(0.2 * series + 0.1 * rap + 0.7 * career)` over
    /// the truncated bytes. A zero denominator gives 0. The `game` sum is computed in `f64`; the width of the
    /// original float is not stated, which can only matter when the sum lands within rounding error of a whole
    /// percent.
    pub fn compute(counts: &CardCounts, ranks: &[u8; 10]) -> ProfileCard {
        let career = percent(
            u64::from(counts.milestones_done) + u64::from(counts.events_done),
            u64::from(counts.milestones_total) + u64::from(counts.events_total),
        );
        let rap = rap_percent(ranks);
        let series = percent(u64::from(counts.series_done), u64::from(counts.series_total));
        let game = (0.2 * f64::from(series) + 0.1 * f64::from(rap) + 0.7 * f64::from(career)) as u8;
        ProfileCard { game, career, rap, series, series_done: counts.series_done, series_total: counts.series_total }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn career_is_the_truncated_share_of_milestones_and_events() {
        let c = CardCounts {
            milestones_done: 10,
            milestones_total: 55,
            events_done: 20,
            events_total: 100,
            ..CardCounts::default()
        };
        // 100 * 30 / 155 = 19.35
        assert_eq!(ProfileCard::compute(&c, &[16; 10]).career, 19);
        let all = CardCounts {
            milestones_done: 55,
            milestones_total: 55,
            events_done: 100,
            events_total: 100,
            ..CardCounts::default()
        };
        assert_eq!(ProfileCard::compute(&all, &[16; 10]).career, 100);
        assert_eq!(ProfileCard::compute(&CardCounts::default(), &[16; 10]).career, 0, "no division by zero");
    }

    #[test]
    fn the_rap_percentage_adds_fifteen_minus_rank_over_140() {
        assert_eq!(rap_percent(&[16; 10]), 0);
        assert_eq!(rap_percent(&[1; 10]), 100);
        assert_eq!(rap_percent(&[15; 10]), 0, "rank 15 adds nothing");
        assert_eq!(rap_percent(&[14, 16, 16, 16, 16, 16, 16, 16, 16, 16]), 0, "1/140 truncates to 0");
        assert_eq!(rap_percent(&[1, 1, 16, 16, 16, 16, 16, 16, 16, 16]), 20, "28 / 140");
        assert_eq!(rap_percent(&[8; 10]), 50, "70 / 140");
        assert_eq!(rap_percent(&[2, 16, 16, 16, 16, 16, 16, 16, 16, 16]), 9, "13 / 140 = 9.28");
    }

    #[test]
    fn the_series_percentage_and_the_pair() {
        let c = CardCounts { series_done: 7, series_total: 44, ..CardCounts::default() };
        let card = ProfileCard::compute(&c, &[16; 10]);
        assert_eq!(card.series, 15);
        assert_eq!((card.series_done, card.series_total), (7, 44));
        assert_eq!(ProfileCard::compute(&CardCounts::default(), &[16; 10]).series, 0);
    }

    #[test]
    fn the_game_percentage_weights_the_truncated_bytes() {
        let full = CardCounts {
            milestones_done: 55,
            milestones_total: 55,
            events_done: 10,
            events_total: 10,
            series_done: 3,
            series_total: 3,
        };
        let card = ProfileCard::compute(&full, &[1; 10]);
        assert_eq!((card.career, card.rap, card.series, card.game), (100, 100, 100, 100));
        let half_career = CardCounts {
            milestones_done: 0,
            milestones_total: 10,
            events_done: 5,
            events_total: 10,
            ..CardCounts::default()
        };
        let card = ProfileCard::compute(&half_career, &[16; 10]);
        assert_eq!((card.career, card.game), (25, 17), "0.7 * 25 = 17.5 truncates to 17");
        let card = ProfileCard::compute(&CardCounts::default(), &[16; 10]);
        assert_eq!(card.game, 0);
    }

    #[test]
    fn series_records_are_filtered_as_the_spec_says() {
        let rec = |named, series, outside| SeriesRecord {
            is_event_19_8_31: named,
            is_challenge_series: series,
            completed_outside_career: outside,
        };
        let records = [rec(false, true, true), rec(false, true, false), rec(true, true, true), rec(false, false, true)];
        assert_eq!(count_series(&records, false, false), (1, 2));
        assert_eq!(count_series(&records, true, false), (0, 0), "the debug-event byte closes the gate");
        assert_eq!(count_series(&records, true, true), (1, 2), "the developer flag reopens it");
    }

    #[test]
    fn lifetime_ranks_use_the_stable_totals_for_statistics_8_and_9() {
        use crate::stable::{CarRecord, CareerRecord};
        let mut stable = Stable::empty();
        stable.cars[0] = CarRecord { handle: 1, flags: 0xF_0002, career_index: 0, ..CarRecord::free() };
        stable.careers[0] = CareerRecord { bounty: 5_000, unserved: [5, 0, 0, 0, 0, 0, 0, 0], ..CareerRecord::new(0) };
        let mut history = PursuitHistory::default();
        history.sums[1] = 25;
        let ranks = lifetime_ranks(&stable, &history);
        assert_eq!(ranks[1], 15, "25 cops involved is the last threshold");
        assert_eq!(ranks[8], 15, "5 infractions is the last threshold");
        assert_eq!(ranks[9], 15, "5,000 bounty is the last threshold");
        assert_eq!(ranks[0], UNRANKED);
    }
}
