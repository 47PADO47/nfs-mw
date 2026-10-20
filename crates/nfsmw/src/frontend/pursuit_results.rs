//! The pursuit result screen (`PostRace_Pursuit.fng`): the numbers of a pursuit challenge's chase, one row each, the row
//! the challenge asked for showing `value / goal` (the package's rows read `00:00.00 / 00:00.00`). The flow opens it with
//! the chase's [`PursuitReport`] when a challenge ends (`post_race.rs`). Continue leaves through the package's own script,
//! whose end (`EXIT_COMPLETE`) ends the race. `docs/provenance/race-mode.md` section 10 has what is known of the package.

use super::ids;
use super::infractions::money;
use super::logic::{Command, Cx, ScreenLogic};
use super::post_race::lap_time;
use crate::viewer::{PursuitReport, RaceAnswer};

/// The label and the value of row `n` (0 to 8) are the name hashes `0x38B136A8 + n` and `0x85AB7369 + n`.
const FIRST_LABEL: u32 = 0x38B1_36A8;
const FIRST_VALUE: u32 = 0x85AB_7369;
/// The heading and its shadow.
const HEADING: u32 = 0xB987_AEA9;
const HEADING_SHADOW: u32 = 0x8481_22A4;
/// The rows the package has past the six this build fills: the unserved infractions, the escapes in a row and the
/// unserved infractions again. They are left empty.
const SPARE_ROWS: std::ops::Range<u32> = 6..9;

/// Shows a [`PursuitReport`].
pub struct PursuitResultsLogic {
    report: PursuitReport,
}

impl PursuitResultsLogic {
    pub fn new(report: PursuitReport) -> Self {
        Self { report }
    }
}

/// The text of row `row` of a report: its value, and `value / goal` on the challenge's own row.
pub fn row_text(report: &PursuitReport, row: usize) -> String {
    let (value, goal): (String, fn(f32) -> String) = match row {
        0 => (lap_time(report.secs), lap_time),
        1 => (report.evasion.map_or_else(|| "--:--.--".to_owned(), lap_time), lap_time),
        2 => (report.cops_damaged.to_string(), count),
        3 => (report.cops_destroyed.to_string(), count),
        4 => (report.roadblocks_dodged.to_string(), count),
        _ => (money(report.cost_to_state), cost),
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
    money(value.round() as u32)
}

impl ScreenLogic for PursuitResultsLogic {
    fn start(&mut self, cx: &mut Cx) {
        // The package's heading reads INFRACTIONS (the screen is shared with the infractions page of a chase).
        cx.text_object(HEADING, "PURSUIT");
        cx.text_object(HEADING_SHADOW, "PURSUIT");
        for row in 0..PursuitReport::ROWS.len() {
            cx.text_object(FIRST_VALUE + row as u32, &row_text(&self.report, row));
        }
        for row in SPARE_ROWS {
            cx.text_object(FIRST_LABEL + row, "");
            cx.text_object(FIRST_VALUE + row, "");
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if message == ids::EXIT_COMPLETE {
            cx.send(Command::RaceAnswer(RaceAnswer::Exit));
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
    fn an_escape_that_did_not_happen_has_no_time() {
        let report = PursuitReport { evasion: None, goal: Some((1, 300.0)), ..PursuitReport::sample() };
        assert_eq!(row_text(&report, 1), "--:--.-- / 05:00.00");
    }
}
