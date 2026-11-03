//! The pursuit result screen (`PostRace_Pursuit.fng`): the numbers of a chase, one row each. Two pages:
//!
//! - a pursuit challenge's, the row the challenge asked for showing `value / goal` (the package's rows read
//!   `00:00.00 / 00:00.00`). The flow opens it with the chase's [`PursuitReport`] when a challenge ends (`post_race.rs`);
//!   Continue ends the race. `docs/provenance/race-mode.md` section 10 has what is known of the package;
//! - the original's pursuit page after an escape in free roam, which the rewards screen's Continue opens: the seven rows
//!   of `docs/specs/pursuit-escape-results.md` section 3, each with its language label. Continue goes back to the game.
//!
//! Continue leaves through the package's own script, whose end (`EXIT_COMPLETE`) answers the screen.

use super::ids;
use super::logic::{Command, Cx, ScreenLogic};
use super::post_race::lap_time;
use crate::i18n::fmt::grouped;
use crate::viewer::{PursuitPage, PursuitReport, RaceAnswer};

/// The label and the value of row `n` (0 to 8) are the name hashes `0x38B136A8 + n` and `0x85AB7369 + n`.
const FIRST_LABEL: u32 = 0x38B1_36A8;
const FIRST_VALUE: u32 = 0x85AB_7369;
/// The heading and its shadow, and the language label they show (`PRP_HEADER_PURSUIT`).
const HEADING: u32 = 0xB987_AEA9;
const HEADING_SHADOW: u32 = 0x8481_22A4;
const HEADING_TEXT: u32 = 0xFEA8_72D4;
/// The rows the package has past the six a challenge fills: the unserved infractions, the escapes in a row and the
/// unserved infractions again. They are left empty.
const SPARE_ROWS: std::ops::Range<u32> = 6..9;
/// The package's rows: the free-roam page leaves the two past its seven empty.
const PACKAGE_ROWS: u32 = 9;
/// The language labels of the free-roam pursuit page's rows, top to bottom **[exe]**: Pursuit Duration, Damaged Police
/// Vehicles, Immobilized Police Vehicles, Spike Strips Dodged, Roadblocks Dodged, Cost to State, Unserved Infractions.
const CHASE_LABELS: [u32; 7] =
    [0x4D64_888D, 0xA999_F6E2, 0x23F6_E732, 0x0291_C816, 0x29DA_BA15, 0xD9BB_7D2D, 0xB7DF_FF96];

/// Shows a [`PursuitReport`].
pub struct PursuitResultsLogic {
    report: PursuitReport,
}

impl PursuitResultsLogic {
    pub fn new(report: PursuitReport) -> Self {
        Self { report }
    }
}

/// The value of row `row` of the free-roam pursuit page: the duration as a time, the rest as grouped numbers.
pub fn chase_row_text(report: &PursuitReport, row: usize) -> String {
    let value = match row {
        0 => return lap_time(report.secs),
        1 => report.cops_damaged,
        2 => report.cops_destroyed,
        3 => report.spike_strips_dodged,
        4 => report.roadblocks_dodged,
        5 => report.cost_to_state,
        6 => report.infractions,
        _ => return String::new(),
    };
    grouped(i64::from(value))
}

/// The text of row `row` of a challenge's report: its value, and `value / goal` on the challenge's own row.
pub fn row_text(report: &PursuitReport, row: usize) -> String {
    let (value, goal): (String, fn(f32) -> String) = match row {
        0 => (lap_time(report.secs), lap_time),
        1 => (report.evasion.map_or_else(|| "--:--.--".to_owned(), lap_time), lap_time),
        2 => (report.cops_damaged.to_string(), count),
        3 => (report.cops_destroyed.to_string(), count),
        4 => (report.roadblocks_dodged.to_string(), count),
        _ => (grouped(i64::from(report.cost_to_state)), cost),
    };
    match report.goal {
        Some((goal_row, target)) if goal_row == row => format!("{value} / {}", goal(target)),
        _ => value,
    }
}

fn count(value: f32) -> String {
    format!("{}", value.round() as u32)
}

fn cost(value: f32) -> String {
    grouped(i64::from(value.round() as u32))
}

impl PursuitResultsLogic {
    /// The free-roam pursuit page: each row's label and value, and the package's spare rows empty.
    fn fill_chase(&self, cx: &mut Cx) {
        for row in 0..PACKAGE_ROWS {
            let label = CHASE_LABELS.get(row as usize).map_or_else(String::new, |&l| crate::i18n::game_or_error(l));
            cx.text_object(FIRST_LABEL + row, &label);
            cx.text_object(FIRST_VALUE + row, &chase_row_text(&self.report, row as usize));
        }
    }
}

impl ScreenLogic for PursuitResultsLogic {
    fn start(&mut self, cx: &mut Cx) {
        // The package's heading reads INFRACTIONS (the screen is shared with the infractions page of a chase).
        let heading = crate::i18n::game_or_error(HEADING_TEXT);
        cx.text_object(HEADING, &heading);
        cx.text_object(HEADING_SHADOW, &heading);
        if self.report.page == PursuitPage::Chase {
            self.fill_chase(cx);
            return;
        }
        for row in 0..PursuitReport::ROWS.len() {
            cx.text_object(FIRST_VALUE + row as u32, &row_text(&self.report, row));
        }
        for row in SPARE_ROWS {
            cx.text_object(FIRST_LABEL + row, "");
            cx.text_object(FIRST_VALUE + row, "");
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if message != ids::EXIT_COMPLETE {
            return;
        }
        match self.report.page {
            PursuitPage::Challenge => cx.send(Command::RaceAnswer(RaceAnswer::Exit)),
            PursuitPage::Chase => cx.send(Command::Resume),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_shows_its_value_and_the_goal_on_the_challenges_own_row() {
        let report = PursuitReport::sample();
        assert_eq!(row_text(&report, 0), "01:35.50");
        assert_eq!(row_text(&report, 2), "12 / 12", "the cops-hit challenge: 12 of 12");
        assert_eq!(row_text(&report, 3), "3");
        assert_eq!(row_text(&report, 5), "25,000");
    }

    #[test]
    fn the_free_roam_page_reads_the_original_rows_in_order() {
        let report = PursuitReport { page: PursuitPage::Chase, cost_to_state: 125_500, ..PursuitReport::sample() };
        let rows: Vec<String> = (0..9).map(|row| chase_row_text(&report, row)).collect();
        assert_eq!(rows, ["01:35.50", "12", "3", "1", "2", "125,500", "4", "", ""]);
    }

    #[test]
    fn an_escape_that_did_not_happen_has_no_time() {
        let report = PursuitReport { evasion: None, goal: Some((1, 300.0)), ..PursuitReport::sample() };
        assert_eq!(row_text(&report, 1), "--:--.-- / 05:00.00");
    }
}
