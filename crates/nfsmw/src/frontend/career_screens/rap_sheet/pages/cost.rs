//! The cost to state page (`RapSheetCTS.fng`): nine lines, each a category with its quantity and cost, and the total
//! cost to state (`docs/specs/career-rapsheet.md` section 7).

use blackbox_feng::ids::PAD_BACK;
use nfsmw_career::rapsheet::{CostRow, Page, RapTotals, cost_rows};

use super::common::{Leave, back_to, header, number, put_label, put_text, put_value};
use crate::career::{Career, CareerHandle};
use crate::frontend::ids::EXIT_COMPLETE;
use crate::frontend::logic::{Cx, ScreenLogic};

/// The name, the bounty and the cost to state header objects.
const NAME: u32 = 0xE3DA_78E7;
const BOUNTY: u32 = 0xE3DA_78E8;
const COST_TO_STATE: u32 = 0x3586_72CC;
/// The line cells `TXT_CATEGORY_n`, `TXT_QUANTITY_n` and `TXT_CASH_n` (n = 1 to 9, each the first hash + n - 1).
const CATEGORY: u32 = 0x3683_DFAC;
const QUANTITY: u32 = 0xD2A1_8F8D;
const CASH: u32 = 0x9068_C46D;

/// The text of each line in page order: the category key, the quantity and the cost.
pub fn line_texts(rows: &[CostRow]) -> Vec<(&'static str, String, String)> {
    rows.iter().map(|row| (row.label, number(u64::from(row.quantity)), number(u64::from(row.cost)))).collect()
}

/// The cost to state page.
pub struct CostPage {
    leave: Leave,
}

impl CostPage {
    pub fn new() -> Self {
        Self { leave: Leave::default() }
    }

    fn render(&self, cx: &mut Cx, career: &Career) {
        let totals = RapTotals::compute(&career.stable, &career.history);
        header(cx, career, NAME);
        put_value(cx, BOUNTY, "RAP_TOTAL_BOUNTY", &number(totals.bounty));
        put_value(cx, COST_TO_STATE, "RAP_TOTAL_COST_STATE", &number(u64::from(totals.cost_to_state)));
        let rows = cost_rows(&career.history);
        for (slot, (label, quantity, cost)) in line_texts(&rows).into_iter().enumerate() {
            let n = slot as u32;
            put_label(cx, CATEGORY + n, label);
            put_text(cx, QUANTITY + n, &quantity);
            put_text(cx, CASH + n, &cost);
        }
    }
}

impl ScreenLogic for CostPage {
    fn start(&mut self, cx: &mut Cx) {
        CareerHandle::shared().with(|career| self.render(cx, career));
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_BACK => self.leave.start(cx, back_to(Page::CostToState)),
            EXIT_COMPLETE => self.leave.finish(cx),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "cost_tests.rs"]
mod tests;
