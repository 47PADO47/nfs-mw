//! The vehicle database (`RapSheetVD.fng`): one row per owned car in car slot order, two rows on the page, the list
//! scrolls with up and down (`docs/specs/career-rapsheet.md` section 6).

use blackbox_feng::ids::{PAD_BACK, PAD_DOWN, PAD_UP};
use blackbox_text::label_hash;
use nfsmw_career::rapsheet::{
    DriveStatus, Page, RapTotals, Step, VehicleRow, preferred_vehicle, recent_vehicle_key, vehicle_rows,
};

use super::common::{Leave, back_to, car_name, header, number, put_label, put_text, put_value};
use crate::career::{Career, CareerHandle};
use crate::frontend::ids::EXIT_COMPLETE;
use crate::frontend::logic::{Cx, ScreenLogic};
use crate::ui::Text;

/// The cells of the two visible rows, `<cell>_n` for n = 1 and 2 (the first hash, then + n - 1).
const CAR: u32 = 0x028C_4F05;
const BOUNTY: u32 = 0x01AE_BC30;
const FINES: u32 = 0xD01E_18C4;
const UNSERVED: u32 = 0x6E5C_90BB;
const TO_DRIVE: u32 = 0xA408_468B;
const EVADED: u32 = 0xB1E7_2F78;
const BUSTED: u32 = 0xCB45_D776;
/// The objects of the page header: the name, the total bounty and the two named cars.
const NAME: u32 = 0x1FFF_B988;
const TOTAL_BOUNTY: u32 = 0x1FFF_B989;
const PREFERRED: u32 = 0x1FFF_B98A;
const RECENT: u32 = 0x1FFF_B98B;

/// Rows on the page at a time.
const VISIBLE: usize = 2;

/// A listed car: its row and the name of its car type (empty when unknown).
pub struct Line {
    pub row: VehicleRow,
    pub name: String,
}

/// The status text key of a row.
pub fn status_key(status: DriveStatus) -> &'static str {
    match status {
        DriveStatus::Impounded => "RAP_VD_IMPOUNDED",
        DriveStatus::ConfirmedToDrive => "RAP_VD_CONFIRMED_TO_DRIVE",
        DriveStatus::SuspectedToDrive => "RAP_VD_SUSPECTED_TO_DRIVE",
    }
}

/// The first row on the page after a step: the list moves one row and stops at the last full page.
pub fn scroll(top: usize, len: usize, step: Step) -> usize {
    match step {
        Step::Up => top.saturating_sub(1),
        Step::Down => (top + 1).min(len.saturating_sub(VISIBLE)),
    }
}

/// The vehicle database page.
pub struct VehiclesPage {
    lines: Vec<Line>,
    top: usize,
    leave: Leave,
}

impl VehiclesPage {
    pub fn new() -> Self {
        Self { lines: Vec::new(), top: 0, leave: Leave::default() }
    }

    fn load(&mut self, cx: &mut Cx) {
        CareerHandle::shared().with(|career| {
            self.lines = listed(career);
            header(cx, career, NAME);
            let totals = RapTotals::compute(&career.stable, &career.history);
            put_value(cx, TOTAL_BOUNTY, "RAP_TOTAL_BOUNTY", &number(totals.bounty));
            let rows: Vec<VehicleRow> = self.lines.iter().map(|line| line.row).collect();
            let preferred = preferred_vehicle(&rows).map(|row| car_name(career, row.frontend_key));
            put_value(cx, PREFERRED, "RAP_PREFERRED_VEHICLE", &car_text(preferred.as_deref()));
            let recent = recent_vehicle_key(&career.history).map(|key| car_name(career, key));
            put_value(cx, RECENT, "RAP_RECENT_VEHICLE", &car_text(recent.as_deref()));
        });
    }

    fn paint(&self, cx: &mut Cx) {
        for slot in 0..VISIBLE {
            let n = slot as u32 + 1;
            let Some(line) = self.lines.get(self.top + slot) else {
                clear(cx, n);
                continue;
            };
            paint_line(cx, n, line);
        }
    }
}

/// The cars of the page: every listed car with its name, in slot order.
fn listed(career: &Career) -> Vec<Line> {
    vehicle_rows(&career.stable).into_iter().map(|row| Line { name: car_name(career, row.frontend_key), row }).collect()
}

/// The car of a header line: its name, `ERROR_DEFAULT_STRING` when the catalogue lacks the car type, and
/// `RAP_VD_NO_CAR_ON_RECORD` when there is no car.
fn car_text(name: Option<&str>) -> String {
    let label = match name {
        None => "RAP_VD_NO_CAR_ON_RECORD",
        Some("") => "ERROR_DEFAULT_STRING",
        Some(name) => return name.to_owned(),
    };
    Text::Game(label_hash(label)).resolve().unwrap_or_default()
}

/// An empty slot: "< no record >" on the car cell, nothing in the others.
fn clear(cx: &mut Cx, n: u32) {
    put_label(cx, CAR + n - 1, "RAP_VD_NO_CAR_ON_RECORD");
    for base in [BOUNTY, FINES, UNSERVED, TO_DRIVE, EVADED, BUSTED] {
        put_text(cx, base + n - 1, "");
    }
}

fn paint_line(cx: &mut Cx, n: u32, line: &Line) {
    let slot = n - 1;
    let row = &line.row;
    match line.name.is_empty() {
        true => put_label(cx, CAR + slot, "ERROR_DEFAULT_STRING"),
        false => put_text(cx, CAR + slot, &line.name),
    }
    put_value(cx, BOUNTY + slot, "RAP_VD_BOUNTY", &number(u64::from(row.bounty)));
    put_value(cx, FINES + slot, "RAP_VD_OUTSTANDING_FINES", &number(u64::from(row.outstanding_fines)));
    put_value(cx, UNSERVED + slot, "RAP_VD_UNSERVED_INFRACTIONS", &number(u64::from(row.unserved)));
    put_label(cx, TO_DRIVE + slot, status_key(row.status));
    put_value(cx, EVADED + slot, "RAP_VD_TIMES_EVADED", &number(u64::from(row.evaded)));
    put_value(cx, BUSTED + slot, "RAP_VD_TIMES_BUSTED", &number(u64::from(row.busted)));
}

impl ScreenLogic for VehiclesPage {
    fn start(&mut self, cx: &mut Cx) {
        self.load(cx);
        self.paint(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_UP => {
                self.top = scroll(self.top, self.lines.len(), Step::Up);
                self.paint(cx);
            }
            PAD_DOWN => {
                self.top = scroll(self.top, self.lines.len(), Step::Down);
                self.paint(cx);
            }
            PAD_BACK => self.leave.start(cx, back_to(Page::Vehicles)),
            EXIT_COMPLETE => self.leave.finish(cx),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "vehicles_tests.rs"]
mod tests;
