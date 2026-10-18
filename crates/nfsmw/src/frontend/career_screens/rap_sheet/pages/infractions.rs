//! The infractions page (`RapSheetUS.fng`): the eight kinds with the count in the chosen set and the total, and the
//! switch key that toggles the unserved and the served set (`docs/specs/career-rapsheet.md` section 5).

use blackbox_feng::ids::PAD_BACK;
use nfsmw_career::rapsheet::{InfractionView, Page, RapTotals, infraction_rows};

use super::common::{Leave, SWITCH_VIEW, back_to, header, number, put_label, put_text, put_value};
use crate::career::{Career, CareerHandle};
use crate::frontend::ids::EXIT_COMPLETE;
use crate::frontend::logic::{Cx, ScreenLogic};

/// The row objects `INFRACTION_TYPE_n`, `UNSERVED_QTY_n` and `TOTAL_QTY_n` (n = 1 to 8, each the first hash + n - 1).
const ROW_LABEL: u32 = 0x7454_3A5D;
const SHOWN: u32 = 0x79CF_1D38;
const TOTAL: u32 = 0xC414_7B50;
/// The column header, `RAP_US_UNSERVED` or `RAP_US_SERVED`.
const COLUMN: u32 = 0x9D97_4DF3;
/// The name, the view's count, the bounty and the view's fines.
const NAME: u32 = 0x1FFF_B988;
const VIEW_TITLE: u32 = 0x1FFF_B989;
const BOUNTY: u32 = 0x1FFF_B98A;
const FINES: u32 = 0x1FFF_B98B;

/// What the header and the column say for a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewHeader {
    /// The title label, with the count in it.
    pub title: &'static str,
    pub count: u64,
    /// The fines label, with the fines in it.
    pub fines: &'static str,
    pub fines_value: u64,
    /// The column header label.
    pub column: &'static str,
}

/// The header of a view: the unserved view shows the fines due, the served view the fines paid.
pub fn view_header(view: InfractionView, totals: &RapTotals) -> ViewHeader {
    match view {
        InfractionView::Unserved => ViewHeader {
            title: "RAP_UNSERVED_INFRACTIONS",
            count: u64::from(totals.unserved),
            fines: "RAP_FINES_DUE",
            fines_value: totals.fines_due,
            column: "RAP_US_UNSERVED",
        },
        InfractionView::Served => ViewHeader {
            title: "RAP_SERVED_INFRACTIONS",
            count: u64::from(totals.served),
            fines: "RAP_FINES_PAID",
            fines_value: totals.fines_paid,
            column: "RAP_US_SERVED",
        },
    }
}

/// The infractions page. It starts on the unserved set; the switch key toggles the set and refills the page.
pub struct InfractionsPage {
    view: InfractionView,
    leave: Leave,
}

impl InfractionsPage {
    pub fn new() -> Self {
        Self { view: InfractionView::Unserved, leave: Leave::default() }
    }

    fn refresh(&self, cx: &mut Cx) {
        CareerHandle::shared().with(|career| self.render(cx, career));
    }

    fn render(&self, cx: &mut Cx, career: &Career) {
        let totals = RapTotals::compute(&career.stable, &career.history);
        header(cx, career, NAME);
        for (i, row) in infraction_rows(&career.stable, self.view).iter().enumerate() {
            let slot = i as u32;
            put_label(cx, ROW_LABEL + slot, row.label);
            put_text(cx, SHOWN + slot, &number(u64::from(row.shown)));
            put_text(cx, TOTAL + slot, &number(u64::from(row.total)));
        }
        let head = view_header(self.view, &totals);
        put_value(cx, VIEW_TITLE, head.title, &number(head.count));
        put_value(cx, FINES, head.fines, &number(head.fines_value));
        put_value(cx, BOUNTY, "RAP_TOTAL_BOUNTY", &number(totals.bounty));
        put_label(cx, COLUMN, head.column);
    }
}

impl ScreenLogic for InfractionsPage {
    fn start(&mut self, cx: &mut Cx) {
        self.refresh(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            SWITCH_VIEW => {
                self.view = self.view.toggled();
                self.refresh(cx);
            }
            PAD_BACK => self.leave.start(cx, back_to(Page::Infractions)),
            EXIT_COMPLETE => self.leave.finish(cx),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "infractions_tests.rs"]
mod tests;
