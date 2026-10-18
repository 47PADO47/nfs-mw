//! The pursuit detail (`RapSheetPD.fng`): one of the top 5 pursuits in full. The entry is the one the top 5 page last
//! opened, read from the screen memory (`docs/specs/career-rapsheet.md` section 8.2). Back returns to the top 5 page.

use blackbox_feng::ids::PAD_BACK;
use nfsmw_career::rapsheet::time::FLAGS_PAGE;
use nfsmw_career::rapsheet::{Page, PursuitEntry, TOP_ENTRIES, format_ticks};

use super::common::{Leave, TOP_ENTRY, back_to, header, number, put_text, put_value, remembered, text_to_nul};
use crate::career::{Career, CareerHandle};
use crate::frontend::ids::EXIT_COMPLETE;
use crate::frontend::logic::{Cx, ScreenLogic};

/// The header name and the objects of the figures (`fe_hash_upper` of their names).
const NAME: u32 = 0xEB40_6FEC;
const CASE_ID: u32 = 0xD919_34E7;
const LENGTH: u32 = 0x9068_C46D;
const COPS_INVOLVED: u32 = 0x9068_C46E;
const COPS_DAMAGED: u32 = 0x9068_C46F;
const COPS_DESTROYED: u32 = 0x9068_C470;
const ROADBLOCKS: u32 = 0x9068_C471;
const SPIKE_STRIPS: u32 = 0x9068_C472;
const COST_TO_STATE: u32 = 0x9068_C473;
const INFRACTIONS: u32 = 0x9068_C474;
const HELICOPTERS: u32 = 0x9068_C475;
const BOUNTY: u32 = 0x9D81_523D;

/// The figures of one entry: each object with its text, the length as a time and the counts as numbers.
pub fn figures(entry: &PursuitEntry) -> Vec<(u32, String)> {
    let count = |value: u32| number(u64::from(value));
    vec![
        (LENGTH, format_ticks(entry.ticks, FLAGS_PAGE)),
        (COPS_INVOLVED, count(entry.cops_involved)),
        (COPS_DAMAGED, count(entry.cops_damaged)),
        (COPS_DESTROYED, count(entry.cops_destroyed)),
        (ROADBLOCKS, count(entry.roadblocks_dodged)),
        (SPIKE_STRIPS, count(entry.spikes_dodged)),
        (COST_TO_STATE, count(entry.cost_to_state)),
        (INFRACTIONS, count(entry.infraction_kinds)),
        (HELICOPTERS, count(entry.helicopters)),
        (BOUNTY, count(entry.bounty)),
    ]
}

/// The index 0 to 4 of the entry the top 5 page opened last (its number minus one). A missing or out of range number
/// reads as entry 1.
pub fn entry_index(remembered_number: Option<usize>) -> usize {
    remembered_number.unwrap_or(1).clamp(1, TOP_ENTRIES) - 1
}

/// The pursuit detail page.
pub struct PursuitDetailPage {
    index: usize,
    leave: Leave,
}

impl PursuitDetailPage {
    pub fn new() -> Self {
        Self { index: 0, leave: Leave::default() }
    }

    fn render(&self, cx: &mut Cx, career: &Career) {
        let Some(entry) = career.history.top.get(self.index) else { return };
        header(cx, career, NAME);
        put_value(cx, CASE_ID, "RAP_PURSUIT_ID", &text_to_nul(&entry.case_id));
        for (name, text) in figures(entry) {
            put_text(cx, name, &text);
        }
    }
}

impl ScreenLogic for PursuitDetailPage {
    fn start(&mut self, cx: &mut Cx) {
        self.index = entry_index(remembered(cx, TOP_ENTRY));
        CareerHandle::shared().with(|career| self.render(cx, career));
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_BACK => self.leave.start(cx, back_to(Page::PursuitDetail(self.index as u8))),
            EXIT_COMPLETE => self.leave.finish(cx),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "pursuit_detail_tests.rs"]
mod tests;
