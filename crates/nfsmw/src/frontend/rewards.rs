//! The milestone rewards screen (`PostRace_MilestoneRewards.fng`): what a chase banked, one row at a time, and the total
//! of the rows shown so far. The flow opens it with the chase's [`RewardsReport`] (`post_race.rs`,
//! `Frontend::poll_race`): after every escape in free roam, and after a race's chase that banked milestones. The package
//! has one text line for the row and one for its bounty, so each row takes its turn once the package's entrance has
//! finished: the row's icon plays its reveal, then moves to its slot in the row of small icons, and the package's message
//! at the end of that brings the next row. When the rows run out the icon goes, the lines leave and both panels slide,
//! which plays the total's flourish. The sounds are the package's own events (`audio/menu.rs`). Continue leaves through
//! the package's own script; its end opens the chase's statistics page when the report has one, else goes back to the
//! game (`docs/specs/pursuit-escape-results.md` section 2).

use blackbox_feng::fe_hash_upper;

use super::ids;
use super::logic::{Command, Cx, ScreenLogic};
use crate::i18n::{self, fmt::grouped};
use crate::viewer::RewardsReport;

/// The milestone's line and its bounty line, and the total at the foot, by name hash.
const MILESTONE_LINE: u32 = 0xE526_D0D2;
const BOUNTY_LINE: u32 = 0xE104_5A4F;
const TOTAL_LINE: u32 = 0x324F_7792;

/// The language labels of the two bounty lines' words, `Bounty` and `Total Bounty` **[exe]**.
const BOUNTY_WORD: u32 = 0x29B1_B96A;
const TOTAL_WORD: u32 = 0x5CCF_949A;

/// The big icon of the row, and its reveal script.
const ICON: u32 = 0xFCC8A;
const REVEAL: u32 = 0x5079_C8F8;
/// The package's message when a row's icon has moved to its slot: the next row comes **[exe]** **[install]**.
const ROW_DONE: u32 = 0x0D3C_3DE7;
/// Seconds of the icon's reveal (3000 FEng ticks at 1200 a second): when it has played, the icon moves to its slot in the
/// row of small icons **[exe]**.
const REVEAL_SECS: f32 = 2.5;
/// Seconds of the slot script, at whose end the package posts [`ROW_DONE`] **[install]**: the wait for a row whose icon
/// has no slot.
const SLOT_SECS: f32 = 0.36;
/// The slot script of each row icon (`POS0` to `POS8`, `POS00`), by the icon's texture **[install]**: the script moves the
/// big icon onto its small icon, lights that one and posts [`ROW_DONE`].
const SLOTS: [(&str, &str); 10] = [
    ("Milestone_Pursuit_Duration", "POS0"),
    ("Cops_TakenOut_Icon", "POS00"),
    ("Milestone_PursuitBounty", "POS1"),
    ("Milestone_Infractions", "POS2"),
    ("Milestone_Cost_To_State", "POS3"),
    ("Milestone_Roadblocks", "POS4"),
    ("Milestone_SpikeStrips", "POS5"),
    ("Cops_Damaged_Icon", "POS6"),
    ("Milestone_Pursuit_MinTime", "POS7"),
    ("Milestone_Pursuit_MaxTime", "POS8"),
];

/// The slot script of a row's icon; `None` for an icon with no slot.
fn slot_of(icon: &str) -> Option<u32> {
    SLOTS.iter().find(|(texture, _)| texture.eq_ignore_ascii_case(icon)).map(|(_, script)| fe_hash_upper(script))
}
/// The script that takes the row's two lines away once the rows have run out **[exe]**.
const LINES_OUT: u32 = 0x0331_13AC;
/// The big icon's hide script, and the two panels with the slide they play once the rows have run out **[exe]**; the
/// upper panel's slide plays the dialog-out sound and dims the Milestones tab.
const HIDE: u32 = 0x0016_A259;
const PANELS: [u32; 2] = [0x962B_9C62, 0xEC85_C7E4];
/// `POS2`, the panels' slide.
const PANEL_SLIDE: u32 = 0x001B_20C3;

/// Shows a [`RewardsReport`].
pub struct RewardsLogic {
    report: RewardsReport,
    shown: usize,
    /// The bounty of the rows shown so far: the total line.
    total: u32,
    /// Seconds since the row came up; `None` once its icon has gone to its slot (or the rows have run out).
    row_age: Option<f32>,
}

impl RewardsLogic {
    pub fn new(report: RewardsReport) -> Self {
        Self { report, shown: 0, total: 0, row_age: None }
    }

    fn fill(&mut self, cx: &mut Cx) {
        let Some(line) = self.report.lines.get(self.shown) else { return };
        // The row's icon takes its texture and plays its reveal: the icon's script posts the messages that bring the
        // row's line, the bounty line and the total in, one after the other.
        if let Some(icon) = cx.guid(ICON) {
            cx.rt.set_texture(icon, fe_hash_upper(line.icon));
            cx.rt.run_script(icon, REVEAL);
        }
        self.row_age = Some(0.0);
        self.total = self.total.saturating_add(line.bounty);
        cx.text_object(MILESTONE_LINE, &line.text);
        // A row that paid nothing has no bounty line **[exe]**.
        let bounty = match line.bounty {
            0 => String::new(),
            paid => format!("{}: {}", i18n::game_or_error(BOUNTY_WORD), grouped(i64::from(paid))),
        };
        cx.text_object(BOUNTY_LINE, &bounty);
        let total = grouped(i64::from(self.total));
        cx.text_object(TOTAL_LINE, &format!("{}: {total}", i18n::game_or_error(TOTAL_WORD)));
    }

    /// The next row, or the lines taken away when the rows have run out.
    fn next(&mut self, cx: &mut Cx) {
        if self.shown + 1 < self.report.lines.len() {
            self.shown += 1;
            self.fill(cx);
            return;
        }
        if self.shown == self.report.lines.len() {
            return;
        }
        self.shown = self.report.lines.len();
        self.row_age = None;
        if let Some(icon) = cx.guid(ICON) {
            cx.rt.run_script(icon, HIDE);
        }
        cx.script(MILESTONE_LINE, LINES_OUT);
        cx.script(BOUNTY_LINE, LINES_OUT);
        for panel in PANELS {
            cx.script(panel, PANEL_SLIDE);
        }
    }

    /// The row's icon goes to its slot once its reveal has played; a row whose icon has no slot goes on by itself.
    fn send_to_slot(&mut self, cx: &mut Cx) {
        self.row_age = None;
        let slot = self.report.lines.get(self.shown).and_then(|line| slot_of(line.icon));
        let (Some(slot), Some(icon)) = (slot, cx.guid(ICON)) else {
            self.next(cx);
            return;
        };
        cx.rt.run_script(icon, slot);
    }
}

impl ScreenLogic for RewardsLogic {
    // The first row waits for the package's entrance: `INIT_COMPLETE`, below.
    fn start(&mut self, _cx: &mut Cx) {}

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        let Some(age) = self.row_age.as_mut() else { return };
        *age += dt;
        let waited = match self.report.lines.get(self.shown).and_then(|line| slot_of(line.icon)) {
            Some(_) => REVEAL_SECS,
            None => REVEAL_SECS + SLOT_SECS,
        };
        if *age >= waited {
            self.send_to_slot(cx);
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            ids::INIT_COMPLETE if self.row_age.is_none() && self.shown == 0 => self.fill(cx),
            ROW_DONE => self.next(cx),
            // The package's own leave animation runs on accept; its end is the way out.
            ids::EXIT_COMPLETE => match self.report.pursuit {
                Some(report) => cx.send(Command::ShowPursuitResults(report)),
                None => cx.send(Command::Resume),
            },
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_row_icon_has_its_slot_script() {
        assert_eq!(slot_of("Milestone_Pursuit_Duration"), Some(0x001B_20C1), "POS0");
        assert_eq!(slot_of("Cops_TakenOut_Icon"), Some(0x037F_3911), "POS00");
        assert_eq!(slot_of("Milestone_Pursuit_MaxTime"), Some(0x001B_20C9), "POS8");
        assert_eq!(slot_of("Milestone_Main"), None);
        assert_eq!(PANEL_SLIDE, fe_hash_upper("POS2"));
        assert_eq!(HIDE, fe_hash_upper("HIDE"));
    }

    #[test]
    fn the_sample_report_is_an_escape_with_its_statistics_page() {
        let report = RewardsReport::sample();
        assert_eq!(report.lines.len(), 3);
        assert_eq!(report.lines.iter().map(|l| l.bounty).sum::<u32>(), 9_000);
        assert!(report.pursuit.is_some_and(|page| page.page == crate::viewer::PursuitPage::Chase));
    }
}
