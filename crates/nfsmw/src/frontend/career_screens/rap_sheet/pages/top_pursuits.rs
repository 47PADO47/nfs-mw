//! The top 5 pursuits (`RapSheetTEP.fng`): five fixed entries sorted by bounty, up and down wrap among the used ones,
//! and Accept opens the pursuit detail of the highlighted entry (`docs/specs/career-rapsheet.md` section 8.1).
//!
//! The entry chosen is kept in the screen memory under [`TOP_ENTRY`] as its number 1 to 5, so the pursuit detail
//! reads it back (`Args` carries no index). Back resets it to 1.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_DOWN, PAD_UP};
use nfsmw_career::rapsheet::time::FLAGS_PAGE;
use nfsmw_career::rapsheet::{
    Page, PursuitEntry, Step, TOP_ENTRIES, format_ticks, highlight_button, top_entry_page, top_step,
};

use super::common::{
    Leave, TOP_ENTRY, back_to, car_name, go, header, number, put_label, put_text, put_value, remember, remembered,
    text_to_nul,
};
use crate::career::{Career, CareerHandle};
use crate::frontend::ids::{EXIT_COMPLETE, SCRIPT_HIGHLIGHT, SCRIPT_UNHIGHLIGHT};
use crate::frontend::logic::{Cx, ScreenLogic};

/// The objects of the entry cells `<cell>_i` (i = 1 to 5, the first hash + i - 1) and the buttons.
const CAR_USED: u32 = 0x273E_2675;
const BOUNTY: u32 = 0x01AE_BC30;
const PURSUIT_ID: u32 = 0x843A_1717;
const PURSUIT_LENGTH: u32 = 0x9EE3_3CEC;
const BUTTON: u32 = 0xCDA0_A66B;
/// The groups with the accept prompts, hidden when no entry is used.
const PROMPTS_A: u32 = 0xEB5E_7757;
const PROMPTS_B: u32 = 0x73DC_B662;
/// The header name and the total bounty.
const NAME: u32 = 0xE3DA_78E7;
const TOTAL_BOUNTY: u32 = 0xE3DA_78E8;

/// One entry as the page shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    pub used: bool,
    pub car: String,
    pub bounty: u64,
    pub case_id: String,
    pub length: String,
}

/// The page's view of a stored entry; the car name comes from the caller.
pub fn entry(row: &PursuitEntry, car: String) -> Entry {
    Entry {
        used: row.is_used(),
        car,
        bounty: u64::from(row.bounty),
        case_id: text_to_nul(&row.case_id),
        length: format_ticks(row.ticks, FLAGS_PAGE),
    }
}

/// Which of the five entries are used, in entry order.
pub fn used_flags(entries: &[Entry]) -> [bool; TOP_ENTRIES] {
    std::array::from_fn(|i| entries.get(i).is_some_and(|entry| entry.used))
}

/// The entries of a history with their car names.
fn entries(career: &Career) -> Vec<Entry> {
    career.history.top.iter().map(|row| entry(row, car_name(career, row.car_key))).collect()
}

/// The top 5 pursuits page.
pub struct TopPursuitsPage {
    entries: Vec<Entry>,
    /// The entry the highlight is on, 1 to 5.
    cursor: usize,
    leave: Leave,
}

impl TopPursuitsPage {
    pub fn new() -> Self {
        Self { entries: Vec::new(), cursor: 1, leave: Leave::default() }
    }

    fn load(&mut self, cx: &mut Cx) {
        CareerHandle::shared().with(|career| {
            self.entries = entries(career);
            header(cx, career, NAME);
            put_value(cx, TOTAL_BOUNTY, "RAP_TOTAL_BOUNTY", &number(career.stable.total_bounty()));
        });
        let byte = remembered(cx, TOP_ENTRY).unwrap_or(1).min(TOP_ENTRIES);
        self.cursor = usize::from(highlight_button(byte as u8));
    }

    fn paint(&self, cx: &mut Cx) {
        let any_used = used_flags(&self.entries).contains(&true);
        for (slot, entry) in self.entries.iter().enumerate() {
            let n = slot as u32;
            paint_entry(cx, n, entry);
            let script = match slot + 1 == self.cursor {
                true => SCRIPT_HIGHLIGHT,
                false => SCRIPT_UNHIGHLIGHT,
            };
            cx.script(BUTTON + n, script);
        }
        cx.hide(PROMPTS_A, !any_used);
        cx.hide(PROMPTS_B, !any_used);
    }

    fn move_cursor(&mut self, cx: &mut Cx, step: Step) {
        let used = used_flags(&self.entries);
        let Some(next) = top_step(&used, self.cursor, step) else { return };
        self.cursor = next;
        self.paint(cx);
    }

    fn accept(&mut self, cx: &mut Cx) {
        let used = used_flags(&self.entries);
        if !used.get(self.cursor - 1).copied().unwrap_or(false) {
            return;
        }
        remember(cx, TOP_ENTRY, self.cursor);
        self.leave.start(cx, go(top_entry_page(self.cursor as u8)));
    }
}

fn paint_entry(cx: &mut Cx, n: u32, entry: &Entry) {
    match entry.used {
        true => {
            put_value(cx, CAR_USED + n, "RAP_TEP_CAR_USED", &entry.car);
            put_value(cx, BOUNTY + n, "RAP_TEP_BOUNTY", &number(entry.bounty));
            put_value(cx, PURSUIT_ID + n, "RAP_TEP_PURSUIT_ID", &entry.case_id);
            put_value(cx, PURSUIT_LENGTH + n, "RAP_TEP_PURSUIT_LENGTH", &entry.length);
        }
        false => {
            put_label(cx, CAR_USED + n, "RAP_TEP_EMPTY_INFRACTION");
            put_text(cx, BOUNTY + n, "");
            put_text(cx, PURSUIT_ID + n, "");
            put_text(cx, PURSUIT_LENGTH + n, "");
        }
    }
}

impl ScreenLogic for TopPursuitsPage {
    fn start(&mut self, cx: &mut Cx) {
        self.load(cx);
        self.paint(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_UP => self.move_cursor(cx, Step::Up),
            PAD_DOWN => self.move_cursor(cx, Step::Down),
            PAD_ACCEPT => self.accept(cx),
            PAD_BACK => {
                remember(cx, TOP_ENTRY, 1);
                self.leave.start(cx, back_to(Page::TopPursuits));
            }
            EXIT_COMPLETE => self.leave.finish(cx),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "top_pursuits_tests.rs"]
mod tests;
