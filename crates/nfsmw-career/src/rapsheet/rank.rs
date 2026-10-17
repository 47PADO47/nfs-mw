//! Rankings: the rank rule, the threshold tables the specs list and the ranking detail rows
//! (`docs/specs/career-rapsheet-2.md` section 1, `docs/specs/career-economy-flows.md` 5.3).
//!
//! The tables are read from the install (`FE_ATTRIB`) by the original; the values listed in the specs are kept here
//! as the defaults and as test data. The rank rule takes the thresholds as a slice, so a caller can pass the
//! install data instead.

/// Number of thresholds in a table.
pub const THRESHOLDS: usize = 15;
/// The rank of a value below every threshold ("UNRANKED").
pub const UNRANKED: u8 = 16;
/// Pursuit duration ticks per second.
pub const TICKS_PER_SECOND: f32 = 4000.0;
/// The factor that converts ticks to seconds for display.
pub const TICKS_TO_SECONDS: f32 = 0.00025;

/// Names of the ten statistics, index order (also the names of the `frontend` collections; a single-pursuit
/// table has the suffix `_in_pursuit`).
pub const STAT_NAMES: [&str; 10] = [
    "pursuit_length",
    "cops_involved",
    "cops_damaged",
    "cops_destroyed",
    "tire_spikes_dodged",
    "roadblocks_dodged",
    "helis_involved",
    "cost_to_state",
    "total_infractions",
    "bounty",
];

/// The statistic behind each of the ten rows of the category page, row order.
pub const CATEGORY_ROWS: [usize; 10] = [0, 1, 2, 3, 5, 4, 7, 8, 6, 9];

/// The detail page title label of each statistic (`RAP_PD_...`, spelled as in the language table), index order.
pub const DETAIL_TITLE_LABELS: [&str; 10] = [
    "RAP_PD_PURSUIT_LENGTH",
    "RAP_PD_COPS_INVOLVED",
    "RAP_PD_COPS_DAMANGED",
    "RAP_PD_COPS_DESTROYED",
    "RAP_PD_SPIKE_STRIPS_DODGED",
    "RAP_PD_ROADBLOCKS_DODGED",
    "RAP_PD_HELICOPTERS_INVOLVED",
    "RAP_PD_COST_TO_STATE",
    "RAP_PD_INFRATIONS_RECORDED",
    "RAP_PD_BOUNTY_ACCUMULATED",
];

/// Lifetime thresholds (rank 1 first), per statistic. Pursuit length is in seconds.
pub const LIFETIME: [[f32; THRESHOLDS]; 10] = [
    [14400., 13200., 12000., 10800., 9600., 8400., 7200., 6000., 4800., 3600., 3000., 2400., 1800., 1200., 600.],
    [1200., 1000., 900., 800., 700., 600., 500., 400., 300., 200., 150., 100., 75., 50., 25.],
    [500., 450., 400., 350., 300., 250., 200., 150., 100., 75., 50., 30., 20., 10., 5.],
    [250., 225., 200., 175., 150., 125., 100., 75., 50., 25., 20., 15., 10., 8., 5.],
    [100., 90., 80., 75., 70., 65., 60., 55., 50., 40., 30., 20., 15., 10., 5.],
    [200., 175., 150., 125., 100., 90., 80., 70., 60., 50., 40., 30., 20., 10., 5.],
    [30., 26., 24., 22., 20., 18., 16., 14., 12., 10., 8., 6., 5., 4., 3.],
    [
        3_000_000., 2_500_000., 2_000_000., 1_500_000., 1_000_000., 750_000., 500_000., 400_000., 300_000., 200_000.,
        150_000., 100_000., 75_000., 50_000., 25_000.,
    ],
    [500., 450., 400., 350., 300., 250., 200., 150., 100., 75., 50., 40., 30., 15., 5.],
    [
        10_000_000.,
        8_000_000.,
        6_000_000.,
        4_000_000.,
        3_000_000.,
        2_000_000.,
        1_000_000.,
        750_000.,
        500_000.,
        250_000.,
        150_000.,
        100_000.,
        50_000.,
        10_000.,
        5_000.,
    ],
];

/// Single-pursuit thresholds (the `*_in_pursuit` tables), per statistic. Pursuit length is in seconds.
pub const SINGLE: [[f32; THRESHOLDS]; 10] = [
    [1800., 1680., 1560., 1200., 900., 600., 540., 480., 420., 360., 300., 240., 180., 120., 60.],
    [150., 125., 100., 75., 50., 30., 25., 20., 15., 12., 10., 8., 6., 4., 2.],
    [50., 45., 40., 35., 30., 25., 20., 15., 12., 10., 8., 6., 4., 2., 1.],
    [50., 45., 40., 35., 30., 25., 20., 18., 15., 12., 10., 8., 6., 4., 2.],
    [30., 25., 20., 18., 16., 14., 12., 10., 8., 6., 5., 4., 3., 2., 1.],
    [30., 26., 24., 22., 20., 18., 16., 14., 12., 10., 8., 6., 4., 2., 1.],
    [5., 4., 4., 4., 4., 3., 3., 3., 3., 2., 2., 2., 1., 1., 1.],
    [
        500_000., 450_000., 400_000., 350_000., 300_000., 250_000., 200_000., 150_000., 125_000., 100_000., 75_000.,
        50_000., 25_000., 15_000., 10_000.,
    ],
    [7., 7., 7., 7., 6., 6., 6., 5., 5., 5., 4., 4., 3., 3., 2.],
    [
        1_500_000., 1_250_000., 1_000_000., 750_000., 500_000., 300_000., 200_000., 150_000., 100_000., 75_000.,
        50_000., 25_000., 10_000., 5_000., 1_000.,
    ],
];

const IN_ORDER: [u8; THRESHOLDS] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

/// The rival numbers whose alias is shown next to each threshold row (`BLACKLIST_RIVAL_<nn>_AKA`), rank 1 first.
/// The lifetime and single tables of a statistic use the same array, except for the bounty.
pub fn rival_order(stat: usize, career: bool) -> [u8; THRESHOLDS] {
    match (stat, career) {
        (1, _) => [11, 2, 3, 4, 15, 6, 7, 8, 9, 10, 1, 12, 13, 14, 5],
        (2 | 3, _) => [14, 1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 2, 15],
        (7, _) => [9, 12, 3, 4, 5, 6, 7, 8, 1, 10, 11, 2, 13, 14, 15],
        (8, _) => [4, 2, 5, 1, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        (9, false) => [1, 5, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 15, 14],
        _ => IN_ORDER,
    }
}

/// The default thresholds of a statistic, or `None` for an index past 9.
pub fn default_thresholds(stat: usize, career: bool) -> Option<&'static [f32; THRESHOLDS]> {
    let tables = match career {
        true => &LIFETIME,
        false => &SINGLE,
    };
    tables.get(stat)
}

/// The rank rule: the 1-based position of the first threshold that is at most the value, 16 when none (or when the
/// table does not hold exactly 15 thresholds). Statistic 0 is stored in ticks of 1/4000 s while its table is in
/// seconds, so its thresholds become `trunc(t * 4000 + 0.5)`; the others are `trunc(t)`. The comparison is a signed
/// 32-bit one: `value` is the u32 sum or best value reinterpreted as `i32`.
pub fn rank(stat: usize, value: u32, thresholds: &[f32]) -> u8 {
    if thresholds.len() != THRESHOLDS {
        return UNRANKED;
    }
    let value = value as i32;
    for (k, &t) in thresholds.iter().enumerate() {
        let limit = match stat {
            0 => (t * TICKS_PER_SECOND + 0.5).trunc() as i32,
            _ => t.trunc() as i32,
        };
        if value >= limit {
            return k as u8 + 1;
        }
    }
    UNRANKED
}

/// The rank of a statistic with the default tables; 16 for an unknown index.
pub fn rank_default(stat: usize, career: bool, value: u32) -> u8 {
    match default_thresholds(stat, career) {
        Some(table) => rank(stat, value, table),
        None => UNRANKED,
    }
}

/// Converts pursuit length ticks to seconds for display (`ticks * 0.00025`).
pub fn ticks_to_seconds(ticks: u32) -> f32 {
    ticks as f32 * TICKS_TO_SECONDS
}

/// One row of the ranking detail page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RankingRow {
    /// The player row: the rank (1 to 16) and the raw value (ticks for statistic 0).
    Player {
        /// The rank; 16 is shown as `--`.
        rank: u8,
    },
    /// A rival row: its rank, the rival number for the alias and the car label, and the table value (seconds for
    /// statistic 0).
    Rival {
        /// The row rank (1 to 15).
        rank: u8,
        /// The rival number.
        rival: u8,
        /// The threshold value.
        threshold: f32,
    },
}

/// The rows of the detail page: 15 rows when the player is ranked (the player replaces the row at his rank, so the
/// 15th threshold is never shown), 16 when unranked (the player row is appended with rank 16).
pub fn ranking_rows(player_rank: u8, thresholds: &[f32; THRESHOLDS], rivals: &[u8; THRESHOLDS]) -> Vec<RankingRow> {
    let rows = match player_rank {
        UNRANKED => THRESHOLDS + 1,
        _ => THRESHOLDS,
    };
    let mut cursor = 0;
    let mut out = Vec::with_capacity(rows);
    for i in 0..rows {
        if i + 1 == usize::from(player_rank) {
            out.push(RankingRow::Player { rank: player_rank });
            continue;
        }
        out.push(RankingRow::Rival { rank: i as u8 + 1, rival: rivals[cursor], threshold: thresholds[cursor] });
        cursor += 1;
    }
    out
}

/// How many times the list scrolls down after it is built so the player row is in view: `rank - 6`, only when
/// positive.
pub fn initial_scroll(player_rank: u8) -> u8 {
    player_rank.saturating_sub(6)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest value that reaches rank k+1 of a table, in the unit the rank rule compares.
    fn limit(stat: usize, t: f32) -> u32 {
        match stat {
            0 => (t * 4000.0 + 0.5).trunc() as u32,
            _ => t as u32,
        }
    }

    #[test]
    fn every_table_has_fifteen_non_increasing_thresholds() {
        for stat in 0..10 {
            for table in [&LIFETIME[stat], &SINGLE[stat]] {
                assert!(table.windows(2).all(|w| w[0] >= w[1]), "stat {stat}");
            }
        }
    }

    #[test]
    fn spot_values_of_the_lifetime_tables_match_the_spec() {
        assert_eq!(LIFETIME[0][0], 14_400.0);
        assert_eq!(LIFETIME[0][14], 600.0);
        assert_eq!(LIFETIME[1][0], 1200.0);
        assert_eq!(LIFETIME[2][13], 10.0);
        assert_eq!(LIFETIME[3][2], 200.0);
        assert_eq!(LIFETIME[4][4], 70.0);
        assert_eq!(LIFETIME[5][5], 90.0);
        assert_eq!(LIFETIME[6][0], 30.0);
        assert_eq!(LIFETIME[7][14], 25_000.0);
        assert_eq!(LIFETIME[8][11], 40.0);
        assert_eq!(LIFETIME[9][0], 10_000_000.0);
        assert_eq!(LIFETIME[9][14], 5_000.0);
    }

    #[test]
    fn spot_values_of_the_single_tables_match_the_spec() {
        assert_eq!(SINGLE[0][..4], [1800.0, 1680.0, 1560.0, 1200.0]);
        assert_eq!(SINGLE[1][0], 150.0);
        assert_eq!(SINGLE[2][14], 1.0);
        assert_eq!(SINGLE[3][7], 18.0);
        assert_eq!(SINGLE[4][3], 18.0);
        assert_eq!(SINGLE[5][1], 26.0);
        assert_eq!(SINGLE[6][..6], [5.0, 4.0, 4.0, 4.0, 4.0, 3.0]);
        assert_eq!(SINGLE[7][8], 125_000.0);
        assert_eq!(SINGLE[8][..4], [7.0; 4]);
        assert_eq!(SINGLE[9][14], 1_000.0);
    }

    #[test]
    fn the_rank_is_the_first_threshold_at_most_the_value_in_every_table() {
        for stat in 0..10 {
            for career in [true, false] {
                let table = default_thresholds(stat, career).unwrap();
                let expected =
                    |v: u32| table.iter().position(|&u| limit(stat, u) <= v).map_or(UNRANKED, |p| p as u8 + 1);
                for &t in table.iter() {
                    let at = limit(stat, t);
                    assert_eq!(rank(stat, at, table), expected(at), "stat {stat} career {career} at {at}");
                    assert_eq!(rank(stat, at - 1, table), expected(at - 1), "stat {stat} career {career} below {at}");
                    assert_eq!(rank(stat, at + 1, table), expected(at + 1));
                }
            }
        }
    }

    #[test]
    fn repeated_thresholds_give_the_first_rank_of_the_run() {
        // helicopters per single pursuit: 5, 4, 4, 4, 4, 3, ...
        assert_eq!(rank(6, 4, &SINGLE[6]), 2);
        assert_eq!(rank(6, 5, &SINGLE[6]), 1);
        assert_eq!(rank(6, 3, &SINGLE[6]), 6);
        // infractions per single pursuit: 7, 7, 7, 7, 6, ...
        assert_eq!(rank(8, 7, &SINGLE[8]), 1);
        assert_eq!(rank(8, 6, &SINGLE[8]), 5);
        assert_eq!(rank(8, 1, &SINGLE[8]), UNRANKED);
    }

    #[test]
    fn boundaries_at_each_threshold_of_a_distinct_table() {
        let table = &LIFETIME[1];
        for (k, &t) in table.iter().enumerate() {
            let v = t as u32;
            assert_eq!(rank(1, v, table), k as u8 + 1, "exactly at threshold {k}");
            let expected_below = if k == 14 { UNRANKED } else { k as u8 + 2 };
            assert_eq!(rank(1, v - 1, table), expected_below, "one below threshold {k}");
        }
        assert_eq!(rank(1, 0, table), UNRANKED);
        assert_eq!(rank(1, u32::MAX, table), UNRANKED, "all comparisons are signed: u32::MAX is -1");
        assert_eq!(rank(1, i32::MAX as u32, table), 1);
    }

    #[test]
    fn pursuit_length_compares_ticks_against_seconds_times_4000() {
        let table = &LIFETIME[0];
        assert_eq!(rank(0, 600 * 4000 - 1, table), UNRANKED, "599.99975 s");
        assert_eq!(rank(0, 600 * 4000, table), 15);
        assert_eq!(rank(0, 14_400 * 4000, table), 1);
        assert_eq!(rank(0, 14_400 * 4000 - 1, table), 2);
        let single = &SINGLE[0];
        assert_eq!(rank(0, 60 * 4000, single), 15);
        assert_eq!(rank(0, 1800 * 4000, single), 1);
    }

    #[test]
    fn a_table_that_is_not_fifteen_long_is_unranked() {
        assert_eq!(rank(1, 1_000_000, &LIFETIME[1][..14]), UNRANKED);
        assert_eq!(rank(1, 1_000_000, &[]), UNRANKED);
        assert_eq!(rank_default(10, true, 1), UNRANKED);
    }

    #[test]
    fn a_fresh_profile_is_unranked_everywhere_that_has_a_positive_last_threshold() {
        for stat in 0..10 {
            assert_eq!(rank_default(stat, true, 0), UNRANKED);
            assert_eq!(rank_default(stat, false, 0), UNRANKED);
        }
    }

    #[test]
    fn category_rows_and_titles_follow_the_page_order() {
        assert_eq!(CATEGORY_ROWS, [0, 1, 2, 3, 5, 4, 7, 8, 6, 9]);
        assert_eq!(STAT_NAMES[CATEGORY_ROWS[4]], "roadblocks_dodged");
        assert_eq!(STAT_NAMES[CATEGORY_ROWS[5]], "tire_spikes_dodged");
        assert_eq!(DETAIL_TITLE_LABELS[2], "RAP_PD_COPS_DAMANGED");
    }

    #[test]
    fn rival_orders_are_permutations_of_one_to_fifteen() {
        for stat in 0..10 {
            for career in [true, false] {
                let mut order = rival_order(stat, career).to_vec();
                order.sort_unstable();
                assert_eq!(order, IN_ORDER, "stat {stat}");
            }
        }
        assert_eq!(rival_order(1, true)[0], 11);
        assert_eq!(rival_order(9, true), IN_ORDER);
        assert_eq!(rival_order(9, false)[..3], [1, 5, 2]);
        assert_eq!(rival_order(9, false)[13..], [15, 14]);
        assert_eq!(rival_order(7, true)[..2], [9, 12]);
    }

    #[test]
    fn a_ranked_player_replaces_his_row_and_the_last_threshold_is_not_shown() {
        let rows = ranking_rows(3, &LIFETIME[1], &rival_order(1, true));
        assert_eq!(rows.len(), 15);
        assert_eq!(rows[2], RankingRow::Player { rank: 3 });
        assert_eq!(rows[0], RankingRow::Rival { rank: 1, rival: 11, threshold: 1200.0 });
        assert_eq!(rows[3], RankingRow::Rival { rank: 4, rival: 3, threshold: 900.0 }, "the cursor skipped no row");
        assert_eq!(rows[14], RankingRow::Rival { rank: 15, rival: 14, threshold: 50.0 });
        assert!(!rows.iter().any(|r| matches!(r, RankingRow::Rival { threshold, .. } if *threshold == 25.0)));
    }

    #[test]
    fn an_unranked_player_is_appended_with_rank_16() {
        let rows = ranking_rows(16, &LIFETIME[9], &rival_order(9, true));
        assert_eq!(rows.len(), 16);
        assert_eq!(rows[15], RankingRow::Player { rank: 16 });
        assert_eq!(rows[14], RankingRow::Rival { rank: 15, rival: 15, threshold: 5000.0 });
    }

    #[test]
    fn the_list_scrolls_so_the_player_row_is_in_view() {
        assert_eq!(initial_scroll(1), 0);
        assert_eq!(initial_scroll(6), 0);
        assert_eq!(initial_scroll(7), 1);
        assert_eq!(initial_scroll(16), 10);
    }

    #[test]
    fn ticks_convert_to_seconds_for_display() {
        assert_eq!(ticks_to_seconds(4000), 1.0);
        assert!((ticks_to_seconds(1000) - 0.25).abs() < 1e-6);
    }
}
