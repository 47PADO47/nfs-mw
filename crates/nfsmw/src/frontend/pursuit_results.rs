//! The pursuit result screen (`PostRace_Pursuit.fng`): the numbers of a chase, one row each. Two kinds of report:
//!
//! - a pursuit challenge's, the row the challenge asked for showing `value / goal` (the package's rows read
//!   `00:00.00 / 00:00.00`). The flow opens it with the chase's [`PursuitReport`] when a challenge ends (`post_race.rs`);
//!   Continue ends the race. `docs/provenance/race-mode.md` section 10 has what is known of the package;
//! - the original's pursuit page after an escape in free roam, which the rewards screen's Continue opens: the seven rows
//!   of `docs/specs/pursuit-escape-results.md` section 3, each with its language label. Continue goes back to the game.
//!
//! The footer's second button (key 2) turns the page **[exe]**: after an escape from the pursuit page to the
//! infractions page, the milestones page and back; after a challenge between the pursuit and the infractions pages
//! (`pursuit_pages.rs`). The package plays the rows out on the key and passes it on to the screen once they are gone,
//! then plays them back in. Continue leaves through the package's own script, which passes the accept on to the screen
//! at its end **[exe]**; the end of both scripts (`EXIT_COMPLETE`) is no answer, as a page turn ends with it too.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BUTTON0};

use super::logic::{Command, Cx, ScreenLogic};
use super::post_race::lap_time;
use super::pursuit_pages::{self, LINES, Line, Mark};
use crate::i18n::fmt::grouped;
use crate::viewer::{PursuitPage, PursuitReport, RaceAnswer};

/// The heading and its shadow.
const HEADING: u32 = 0xB987_AEA9;
const HEADING_SHADOW: u32 = 0x8481_22A4;
/// The footer button that turns the page: its string, which names the page it goes to.
const PAGE_BUTTON: u32 = 0xC6BE_1C1F;
/// What the package passes on once the rows have gone: the pad's fifth button, key 2.
const TURN_PAGE: u32 = PAD_BUTTON0 + 4;
/// The headings `PURSUIT`, `INFRACTIONS` and `MILESTONES`, and the button's `Infractions`, `Milestones` and `Pursuit`
/// **[exe]**.
const PURSUIT_HEADING: u32 = 0xFEA8_72D4;
const INFRACTIONS_HEADING: u32 = 0xFB41_5E78;
const MILESTONES_HEADING: u32 = 0x578B_767B;
const TO_INFRACTIONS: u32 = 0xAEAE_B62F;
const TO_MILESTONES: u32 = 0x7441_3352;
const TO_PURSUIT: u32 = 0x7448_870B;
/// The rows a challenge fills; the package's three past them stay empty.
const CHALLENGE_ROWS: usize = 6;
/// The language labels of the free-roam pursuit page's rows, top to bottom **[exe]**: Pursuit Duration, Damaged Police
/// Vehicles, Immobilized Police Vehicles, Spike Strips Dodged, Roadblocks Dodged, Cost to State, Unserved Infractions.
const CHASE_LABELS: [u32; 7] =
    [0x4D64_888D, 0xA999_F6E2, 0x23F6_E732, 0x0291_C816, 0x29DA_BA15, 0xD9BB_7D2D, 0xB7DF_FF96];

/// The page the screen shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shown {
    Pursuit,
    Infractions,
    Milestones,
}

/// Shows a [`PursuitReport`].
pub struct PursuitResultsLogic {
    report: PursuitReport,
    shown: Shown,
}

impl PursuitResultsLogic {
    pub fn new(report: PursuitReport) -> Self {
        Self { report, shown: Shown::Pursuit }
    }

    /// The page after this one: a challenge has no milestones page.
    fn next(&self) -> Shown {
        match (self.shown, self.report.page) {
            (Shown::Pursuit, _) => Shown::Infractions,
            (Shown::Infractions, PursuitPage::Chase) => Shown::Milestones,
            (Shown::Infractions, PursuitPage::Challenge) | (Shown::Milestones, _) => Shown::Pursuit,
        }
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
    /// The page shown: its heading, the button's name of the next page, and its lines.
    fn show(&self, cx: &mut Cx) {
        let heading = crate::i18n::game_or_error(match self.shown {
            Shown::Pursuit => PURSUIT_HEADING,
            Shown::Infractions => INFRACTIONS_HEADING,
            Shown::Milestones => MILESTONES_HEADING,
        });
        cx.text_object(HEADING, &heading);
        cx.text_object(HEADING_SHADOW, &heading);
        let button = match self.next() {
            Shown::Pursuit => TO_PURSUIT,
            Shown::Infractions => TO_INFRACTIONS,
            Shown::Milestones => TO_MILESTONES,
        };
        cx.text_object(PAGE_BUTTON, &crate::i18n::game_or_error(button));
        let lines = match self.shown {
            Shown::Pursuit => self.pursuit_lines(),
            Shown::Infractions => pursuit_pages::infractions(&self.report.committed),
            Shown::Milestones => pursuit_pages::milestones(&self.report.milestones),
        };
        for n in 0..LINES {
            pursuit_pages::draw(cx, n, lines.get(n as usize));
        }
    }

    /// The pursuit page: the free-roam rows with their labels, or a challenge's rows under the package's own labels.
    fn pursuit_lines(&self) -> Vec<Line> {
        let line = |label: String, value: String| Line { label, value: Some(value), mark: Mark::None };
        if self.report.page == PursuitPage::Chase {
            return CHASE_LABELS
                .iter()
                .enumerate()
                .map(|(row, &label)| line(crate::i18n::game_or_error(label), chase_row_text(&self.report, row)))
                .collect();
        }
        PursuitReport::ROWS
            .iter()
            .take(CHALLENGE_ROWS)
            .enumerate()
            .map(|(row, &label)| line(label.to_owned(), row_text(&self.report, row)))
            .collect()
    }
}

impl ScreenLogic for PursuitResultsLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.show(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if message == TURN_PAGE {
            self.shown = self.next();
            self.show(cx);
            return;
        }
        if message != PAD_ACCEPT {
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

    #[test]
    fn key_2_turns_an_escapes_pages_through_infractions_and_milestones_and_a_challenges_through_infractions() {
        let mut chase = PursuitResultsLogic::new(PursuitReport { page: PursuitPage::Chase, ..PursuitReport::sample() });
        let mut seen = Vec::new();
        for _ in 0..3 {
            chase.shown = chase.next();
            seen.push(chase.shown);
        }
        assert_eq!(seen, [Shown::Infractions, Shown::Milestones, Shown::Pursuit]);
        let mut challenge = PursuitResultsLogic::new(PursuitReport::sample());
        challenge.shown = challenge.next();
        assert_eq!((challenge.shown, challenge.next()), (Shown::Infractions, Shown::Pursuit));
    }
}
