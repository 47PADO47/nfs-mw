//! The summary page (`RapSheetRS.fng`): the career's totals and the police verdict line, chosen by the number of
//! infractions (`docs/specs/career-rapsheet.md` section 4). Nothing changes on the page; Back returns to the main page.

use blackbox_feng::ids::PAD_BACK;
use nfsmw_career::rapsheet::{Page, RapTotals, summary_label};

use super::common::{Leave, back_to, header, number, put_label, put_value};
use crate::career::{Career, CareerHandle};
use crate::frontend::ids::EXIT_COMPLETE;
use crate::frontend::logic::{Cx, ScreenLogic};

/// The objects of the page (`fe_hash_upper` of their names), with the labels they show.
const NAME: u32 = 0xB259_EEA7;
const BOUNTY: u32 = 0xB259_EEA8;
const COST_TO_STATE: u32 = 0xB259_EEA9;
const CARS_IMPOUNDED: u32 = 0xB259_EEAA;
const UNSERVED: u32 = 0x6EB5_AC50;
const SERVED: u32 = 0x6EB5_AC51;
const TIMES_EVADED: u32 = 0xD919_049F;
const TIMES_BUSTED: u32 = 0xD919_04A0;
const FINES_DUE: u32 = 0xD01E_18C5;
const FINES_PAID: u32 = 0xD01E_18C6;
const POLICE_DAMAGED: u32 = 0xD7E5_D0CC;
const POLICE_DESTROYED: u32 = 0xD7E5_D0CD;
/// The verdict line, `RAP_SUMMARY_nn`.
const VERDICT: u32 = 0x9021_1462;

/// The summary page.
pub struct SummaryPage {
    leave: Leave,
}

impl SummaryPage {
    pub fn new() -> Self {
        Self { leave: Leave::default() }
    }

    fn render(&self, cx: &mut Cx, career: &Career) {
        let totals = RapTotals::compute(&career.stable, &career.history);
        header(cx, career, NAME);
        put_value(cx, BOUNTY, "RAP_TOTAL_BOUNTY", &number(totals.bounty));
        put_value(cx, COST_TO_STATE, "RAP_TOTAL_COST_STATE", &number(u64::from(totals.cost_to_state)));
        put_value(cx, CARS_IMPOUNDED, "RAP_TOTAL_CARS_IMPOUNDED", &number(totals.cars_impounded as u64));
        put_value(cx, UNSERVED, "RAP_RS_UNSERVED_INFRACTIONS", &number(u64::from(totals.unserved)));
        put_value(cx, SERVED, "RAP_RS_SERVED_INFRACTIONS", &number(u64::from(totals.served)));
        put_value(cx, TIMES_EVADED, "RAP_RS_TIMES_EVADED", &number(u64::from(totals.times_evaded)));
        put_value(cx, TIMES_BUSTED, "RAP_RS_TIMES_BUSTED", &number(u64::from(totals.times_busted)));
        put_value(cx, FINES_DUE, "RAP_RS_FINES_DUE", &number(totals.fines_due));
        put_value(cx, FINES_PAID, "RAP_RS_FINES_PAID", &number(totals.fines_paid));
        put_value(cx, POLICE_DAMAGED, "RAP_RS_POLICE_DAMAGED", &number(u64::from(totals.police_damaged)));
        put_value(cx, POLICE_DESTROYED, "RAP_RS_POLICE_DESTROYED", &number(u64::from(totals.police_destroyed)));
        put_label(cx, VERDICT, &summary_label(totals.verdict()));
    }
}

impl ScreenLogic for SummaryPage {
    fn start(&mut self, cx: &mut Cx) {
        CareerHandle::shared().with(|career| self.render(cx, career));
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_BACK => self.leave.start(cx, back_to(Page::Summary)),
            EXIT_COMPLETE => self.leave.finish(cx),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "summary_tests.rs"]
mod tests;
