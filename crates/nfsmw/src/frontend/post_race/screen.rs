//! The screen's logic: it copies the rows of the page into the package's strings, and answers the buttons. Exit goes on
//! in free roam (or back to the menu the race came from), Restart runs the race again, the second button (key 2) shows
//! the next page, the shoulder buttons step through the racers of the statistics and stages pages and Back goes back a page
//! (from the standings of a win, back to the photo). A race the player did not win asks for the retry when Exit is
//! answered (`retry.rs`, `docs/specs/race-loss-retry.md`).

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BUTTON0, PAD_LTRIGGER, PAD_RTRIGGER};
use blackbox_feng::runtime::pad;

use super::pages::{Cell, Page, Table, table};
use super::retry;
use crate::frontend::confirm::Confirm;
use crate::frontend::logic::{Command, Cx, ScreenLogic};
use crate::ui::Text;
use crate::viewer::{RaceAnswer, RaceReport};
use nfsmw_career::story::LossRule;

/// The racer's name over the table: the group whose two strings (shadow and face) hold it, inside the group of the arrows.
const NAME_GROUP: u32 = 0xEB43_CCB0;
const FILTER_GROUP: u32 = 0x8A41_F5B9;
/// The title, and the footer's second button (its string is called `Stats` in the package).
const TITLE_GROUP: u32 = 0x2D69_1760;
const FOOTER_BUTTON: u32 = 0x6F23_15C4;
/// The column headers, left to right.
const HEADERS: [u32; 3] = [0x586A_B4A6, 0x44AC_8987, 0x30EE_5E68];
/// The table's rows, by the name hashes of their strings: the left cell, the middle cell and the right cell. The strings
/// of row `n` (1 to 9) are consecutive hashes from the first row's; the tenth and the eleventh are named on their own.
const FIRST_LEFT: u32 = 0x38B1_36A8;
const FIRST_MIDDLE: u32 = 0x85AB_7369;
const FIRST_RIGHT: u32 = 0xD2A5_B02A;
/// The groups of the rows 1 to 10, top to bottom (the package's draw order is not the row order).
const ROW_GROUPS: [u32; 10] = [
    0x57F1_7624,
    0xA4EB_B2E5,
    0xF1E5_EFA6,
    0x3EE0_2C67,
    0x8BDA_6928,
    0xD8D4_A5E9,
    0x25CE_E2AA,
    0x72C9_1F6B,
    0xBFC3_5C2C,
    0x6B65_6154,
];
/// The strings of the tenth row (left, middle, right), and the group of the eleventh row, which stays hidden.
const TENTH: [u32; 3] = [0x4ED8_0BD8, 0x3B19_E0B9, 0x275B_B59A];
const ELEVENTH_GROUP: u32 = 0xB85F_9E15;
/// The buttons this build has no screen for (the console footer's stats, the online gamer card, mute and feedback),
/// hidden, by GUID.
const UNUSED: [u32; 5] = [0xE690F, 0xFA5B3, 0xFD000, 0x10688A, 0x122661];
/// The footer's second button (the group of the `Stats` string and the key 2 glyph), by GUID.
const STATS_BUTTON: u32 = 0x1062F9;
/// The pad bit of the second button (key 2, the pad's fourth button).
const SECOND_BUTTON: u32 = pad::BUTTON0 << 4;
/// The pad bits of key 1, Restart: the lap table answers the first and the photo the second.
const RESTART_BUTTONS: u32 = (pad::BUTTON0 << 1) | (pad::BUTTON0 << 5);

/// Shows a [`RaceReport`].
pub struct PostRaceLogic {
    report: RaceReport,
    page: Page,
    shown: usize,
    /// What the page being shown asks the frame to hold: the rows and headers in use, and the racer filter.
    rows: usize,
    headers: [bool; 3],
    filter: bool,
    /// The buttons that already acted in this frame, as pad bits: one press can reach the screen as a pad bit, as a message and as
    /// the press of a footer button, and must act once.
    done: u32,
}

impl PostRaceLogic {
    pub fn new(report: RaceReport) -> Self {
        Self { report, page: Page::Standings, shown: 0, rows: 0, headers: [true; 3], filter: false, done: 0 }
    }

    /// Looks at the racer `step` places on, wrapping: only the pages that have a racer filter step.
    fn step(&mut self, cx: &mut Cx, step: isize) {
        let n = self.report.racers.len() as isize;
        if n <= 1 || self.page == Page::Standings {
            return;
        }
        self.shown = (self.shown as isize + step).rem_euclid(n) as usize;
        self.fill(cx);
    }

    fn advance(&mut self, cx: &mut Cx) {
        self.page = self.page.next();
        self.fill(cx);
    }

    /// Whether the button `bit` has not acted yet in this frame; the first ask takes it (`done`).
    fn once(&mut self, bit: u32) -> bool {
        let first = self.done & bit == 0;
        self.done |= bit;
        first
    }

    /// Exit was answered: the race ends, or a race that was not won asks for the retry first.
    fn leave(&mut self, cx: &mut Cx) {
        cx.send(leave_command(self.report.retry));
    }

    /// Key 1: the in-race question, over the screen.
    fn ask_restart(&mut self, cx: &mut Cx) {
        if self.once(RESTART_BUTTONS) {
            cx.send(Command::Confirm(Confirm::restart_results()));
        }
    }

    /// Back: the page before, or from the first page of a win the photo the screen was opened over.
    fn back(&mut self, cx: &mut Cx) {
        match back_target(self.page, self.report.photo.is_some()) {
            Back::Page(page) => {
                self.page = page;
                self.fill(cx);
            }
            Back::Photo => cx.send(Command::ShowPhoto(self.report.clone())),
            Back::Stay => {}
        }
    }

    fn fill(&mut self, cx: &mut Cx) {
        let units = cx.settings.speed_units;
        let page = table(&self.report, self.page, self.shown, units);
        self.apply(cx, &page);
        if let Some(racer) = self.report.racers.get(self.shown) {
            cx.text_group(NAME_GROUP, &racer.name);
        }
    }

    fn apply(&mut self, cx: &mut Cx, page: &Table) {
        text_group(cx, TITLE_GROUP, page.title);
        text_object(cx, FOOTER_BUTTON, page.footer);
        for (hash, label) in HEADERS.iter().zip(page.headers) {
            if let Some(label) = label {
                text_object(cx, *hash, label);
            }
        }
        self.headers = page.headers.map(|h| h.is_some());
        self.filter = page.filter;
        self.rows = page.rows.len().min(ROW_GROUPS.len());
        for (row, cells) in page.rows.iter().take(ROW_GROUPS.len()).enumerate() {
            for (hash, cell) in names(row).into_iter().zip(cells) {
                let text = resolve(cx, cell);
                cx.text_object(hash, &text);
            }
        }
        self.hold(cx);
    }

    /// Keeps the objects of the page that is not shown hidden, and the ones this build has no use for.
    fn hold(&self, cx: &mut Cx) {
        for (row, group) in ROW_GROUPS.iter().enumerate() {
            cx.hide(*group, row >= self.rows);
        }
        for (hash, shown) in HEADERS.iter().zip(self.headers) {
            cx.hide(*hash, !shown);
        }
        cx.hide(FILTER_GROUP, !self.filter);
        if let Some(button) = cx.guid(STATS_BUTTON) {
            cx.rt.set_hidden(button, false);
        }
        cx.hide(ELEVENTH_GROUP, true);
        for guid in UNUSED {
            let Some(o) = cx.guid(guid) else { continue };
            cx.rt.set_hidden(o, true);
        }
    }
}

/// A language label as text; empty when the table lacks it.
fn label_text(cx: &Cx, label: u32) -> String {
    Text::Game(label).resolve(cx.assets).unwrap_or_default()
}

fn text_group(cx: &mut Cx, group: u32, label: u32) {
    let text = label_text(cx, label);
    cx.text_group(group, &text);
}

fn text_object(cx: &mut Cx, name: u32, label: u32) {
    let text = label_text(cx, label);
    cx.text_object(name, &text);
}

/// What a cell reads.
fn resolve(cx: &Cx, cell: &Cell) -> String {
    match cell {
        Cell::Empty => String::new(),
        Cell::Text(text) => text.clone(),
        Cell::Label(label) => label_text(cx, *label),
        Cell::WithUnit(number, unit) => format!("{number} {}", label_text(cx, *unit)),
    }
}

/// Every object (by name hash) the logic sets or hides, for the install test.
#[cfg(test)]
pub(super) fn used_objects() -> Vec<u32> {
    let mut all = vec![NAME_GROUP, FILTER_GROUP, TITLE_GROUP, FOOTER_BUTTON, ELEVENTH_GROUP];
    all.extend(HEADERS);
    all.extend(ROW_GROUPS);
    for row in 0..ROW_GROUPS.len() {
        all.extend(names(row));
    }
    all
}

/// The name hashes of the strings of row `row` (from 0): left, middle, right.
fn names(row: usize) -> [u32; 3] {
    if row >= 9 {
        return TENTH;
    }
    let n = row as u32;
    [FIRST_LEFT + n, FIRST_MIDDLE + n, FIRST_RIGHT + n]
}

/// What Back does on a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Back {
    /// Show this page again.
    Page(Page),
    /// Show the win photo again.
    Photo,
    /// Nothing: a race that was not won has no screen before its results.
    Stay,
}

fn back_target(page: Page, has_photo: bool) -> Back {
    if page != Page::Standings {
        return Back::Page(page.previous());
    }
    if has_photo {
        return Back::Photo;
    }
    Back::Stay
}

/// What answering Exit asks of the front end, by what the race leads to (`LossRule`).
fn leave_command(rule: LossRule) -> Command {
    match rule {
        LossRule::Leave => Command::RaceAnswer(RaceAnswer::Exit),
        LossRule::Offer => Command::Confirm(retry::offer()),
        LossRule::Require => Command::Alert(retry::required()),
    }
}

/// Whether the object called `name` lies inside the group with this GUID (or is it): the buttons of the footer are groups
/// holding a string and a key picture.
fn lies_inside(cx: &Cx, name: u32, group: u32) -> bool {
    let Some(package) = cx.rt.package(cx.package) else { return false };
    let Some(first) = cx.object(name) else { return false };
    let mut index = first.index;
    for _ in 0..package.objects.len() {
        let Some(def) = package.objects.get(index) else { return false };
        if def.guid == group {
            return true;
        }
        let Some(parent) = def.parent else { return false };
        let Some(next) = package.objects.iter().position(|o| o.guid == parent) else { return false };
        index = next;
    }
    false
}

impl ScreenLogic for PostRaceLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.fill(cx);
    }

    /// The package's scripts show these again as the screen opens, so they are held every frame. A button the package pressed
    /// switched its inputs off as if the screen were leaving; this screen stays until the flow clears it, so they come back.
    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.done = 0;
        self.hold(cx);
        cx.rt.set_input_enabled(cx.package, true);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_ACCEPT | PAD_BUTTON0 if self.once(pad::ACCEPT) => self.leave(cx),
            m if m == PAD_BUTTON0 + 1 => self.ask_restart(cx),
            PAD_LTRIGGER if self.once(pad::LTRIGGER) => self.step(cx, -1),
            PAD_RTRIGGER if self.once(pad::RTRIGGER) => self.step(cx, 1),
            _ => {}
        }
    }

    /// A button of the footer was pressed with the accept button or a click: the second one shows the next page. Exit and
    /// Restart reach the screen as messages.
    fn pressed(&mut self, cx: &mut Cx, button: u32) {
        let stats = button == STATS_BUTTON || lies_inside(cx, FOOTER_BUTTON, button);
        if stats && self.once(SECOND_BUTTON) {
            self.advance(cx);
        }
    }

    /// The buttons that act whether or not the package answers them or has the focus on the matching footer button: the second
    /// one shows the next page, key 1 asks about the restart, the shoulders step the racers and back goes back a page (or, from the
    /// win photo's Race Stats, back to the photo).
    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        if bits & pad::BACK != 0 {
            self.back(cx);
        }
        if bits & SECOND_BUTTON != 0 && self.once(SECOND_BUTTON) {
            self.advance(cx);
        }
        if bits & RESTART_BUTTONS != 0 {
            self.ask_restart(cx);
        }
        if bits & pad::LTRIGGER != 0 && self.once(pad::LTRIGGER) {
            self.step(cx, -1);
        }
        if bits & pad::RTRIGGER != 0 && self.once(pad::RTRIGGER) {
            self.step(cx, 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_name_consecutive_strings_up_to_the_ninth() {
        assert_eq!(names(0), [0x38B1_36A8, 0x85AB_7369, 0xD2A5_B02A]);
        assert_eq!(names(8), [0x38B1_36B0, 0x85AB_7371, 0xD2A5_B032]);
        assert_eq!(names(9), [0x4ED8_0BD8, 0x3B19_E0B9, 0x275B_B59A]);
    }

    #[test]
    fn the_logic_starts_on_the_standings_of_the_player() {
        let logic = PostRaceLogic::new(RaceReport::sample());
        assert_eq!((logic.page, logic.shown), (Page::Standings, 0));
    }

    #[test]
    fn a_button_acts_once_in_a_frame_however_many_ways_it_arrives() {
        let mut logic = PostRaceLogic::new(RaceReport::sample());
        assert!(logic.once(SECOND_BUTTON), "the pad's bit");
        assert!(!logic.once(SECOND_BUTTON), "the footer button's press in the same frame");
        assert!(logic.once(pad::LTRIGGER), "another button is its own");
        logic.done = 0;
        assert!(logic.once(SECOND_BUTTON), "the next frame");
    }

    #[test]
    fn leaving_a_race_that_was_not_won_asks_what_the_rule_says() {
        assert_eq!(leave_command(LossRule::Leave), Command::RaceAnswer(RaceAnswer::Exit));
        assert_eq!(leave_command(LossRule::Offer), Command::Confirm(retry::offer()));
        assert_eq!(leave_command(LossRule::Require), Command::Alert(retry::required()));
    }

    #[test]
    fn back_steps_to_the_page_before_and_from_the_standings_to_the_photo() {
        assert_eq!(back_target(Page::Stages, true), Back::Page(Page::Statistics));
        assert_eq!(back_target(Page::Statistics, false), Back::Page(Page::Standings));
        assert_eq!(back_target(Page::Standings, true), Back::Photo, "Race Stats opened it over the photo");
        assert_eq!(back_target(Page::Standings, false), Back::Stay, "a lost race has no photo to go back to");
    }
}
