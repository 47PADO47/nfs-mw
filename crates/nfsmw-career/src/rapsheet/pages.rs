//! The numbers on the rap sheet pages (`docs/specs/career-rapsheet.md` sections 1.4 and 3 to 8): the shared stable
//! totals, the summary verdict index, the infractions rows, the vehicle database rows, the cost to state rows and
//! the top 5 pursuits navigation. Text comes from the language table by label; only keys are given here.

use super::history::{PursuitEntry, PursuitHistory, TOP_ENTRIES};
use crate::economy::{InfractionKind, fine_of_counters};
use crate::stable::{CareerRecord, InfractionSet, Stable};

/// The install value of the `rapsheet_summary_strings` array.
pub const SUMMARY_THRESHOLDS: [u32; 5] = [0, 100, 400, 800, 1000];

/// The printed index `nn` of `RAP_SUMMARY_nn`: the first position `i` of `thresholds` with `total <= T[i]`, or
/// `N` (the length) when none matches; a position of 0 prints as 1. With the install array: 0 to 100 infractions
/// give 1, 101 to 400 give 2, 401 to 800 give 3, 801 to 1000 give 4, more give 5. `total` is the served plus the
/// unserved infractions of the whole stable.
pub fn summary_index(total: u32, thresholds: &[u32]) -> usize {
    let nn = thresholds.iter().position(|&t| total <= t).unwrap_or(thresholds.len());
    match nn {
        0 => 1,
        n => n,
    }
}

/// The label key of the verdict: `RAP_SUMMARY_%.2d`.
pub fn summary_label(nn: usize) -> String {
    format!("RAP_SUMMARY_{nn:02}")
}

/// The figures shared by the Main and Summary pages, computed from the stable and the history (never stored).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RapTotals {
    /// Sold-history bounty plus the bounty of every live car.
    pub bounty: u64,
    /// Statistic 7, cost to state (the history sum, not the sum of the nine cost lines).
    pub cost_to_state: u32,
    /// Live cars whose impound state is not 0.
    pub cars_impounded: usize,
    /// Live cars.
    pub cars_monitored: usize,
    /// Fines due: live cars only, no sold history.
    pub fines_due: u64,
    /// Fines paid at today amounts: live cars only.
    pub fines_paid: u64,
    /// Times evaded, sold history included.
    pub times_evaded: u32,
    /// Times busted, sold history included.
    pub times_busted: u32,
    /// All unserved infractions, sold history included.
    pub unserved: u32,
    /// All served infractions, sold history included.
    pub served: u32,
    /// Statistic 2, police vehicles damaged.
    pub police_damaged: u32,
    /// Statistic 3, police vehicles destroyed ("immobilized").
    pub police_destroyed: u32,
}

impl RapTotals {
    /// Computes the figures.
    pub fn compute(stable: &Stable, history: &PursuitHistory) -> RapTotals {
        RapTotals {
            bounty: stable.total_bounty(),
            cost_to_state: history.sums[7],
            cars_impounded: stable.cars_impounded(),
            cars_monitored: stable.cars_monitored(),
            fines_due: stable.fines_due(),
            fines_paid: stable.fines_paid(),
            times_evaded: stable.total_evaded(),
            times_busted: stable.total_busted(),
            unserved: stable.infraction_set_total(InfractionSet::Unserved),
            served: stable.infraction_set_total(InfractionSet::Served),
            police_damaged: history.sums[2],
            police_destroyed: history.sums[3],
        }
    }

    /// All infractions, served plus unserved: the number the verdict is chosen by.
    pub const fn all_infractions(&self) -> u32 {
        self.unserved + self.served
    }

    /// The verdict index with the install thresholds.
    pub fn verdict(&self) -> usize {
        summary_index(self.all_infractions(), &SUMMARY_THRESHOLDS)
    }
}

/// The statistics accessor: 0 to 7 the raw sums, 8 all infractions of the stable, 9 the stable bounty (saturated
/// to 32 bits). `None` past 9.
pub fn statistic(stable: &Stable, history: &PursuitHistory, index: usize) -> Option<u32> {
    match index {
        0..=7 => Some(history.sums[index]),
        8 => Some(stable.total_infractions()),
        9 => Some(u32::try_from(stable.total_bounty()).unwrap_or(u32::MAX)),
        _ => None,
    }
}

/// Which set the Infractions page shows. The page starts on unserved; the key toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfractionView {
    /// View flag 1 (the start value).
    Unserved,
    /// View flag 0.
    Served,
}

impl InfractionView {
    /// The other view.
    pub const fn toggled(self) -> InfractionView {
        match self {
            InfractionView::Unserved => InfractionView::Served,
            InfractionView::Served => InfractionView::Unserved,
        }
    }

    const fn set(self) -> InfractionSet {
        match self {
            InfractionView::Unserved => InfractionSet::Unserved,
            InfractionView::Served => InfractionSet::Served,
        }
    }
}

/// The row label keys of the Infractions page, by counter index. Rows 1 and 2 are swapped against the pursuit HUD
/// naming (`RACING` labels the speeding counter); the original is kept (unconfirmed whether intended).
pub const INFRACTION_ROW_LABELS: [&str; 8] = [
    "RAP_US_RACING",
    "RAP_US_EXCESSIVE_SPEEDING",
    "RAP_US_RECKLESS_DRIVING",
    "RAP_US_ASSULTING_POLICE_OFFICER",
    "RAP_US_HIT_AND_RUN",
    "RAP_US_DAMAGE_TO_PROPERTY",
    "RAP_US_RESISTING_ARREST",
    "RAP_US_DRIVING_OFF_ROADWAY",
];

/// One row of the Infractions page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InfractionRow {
    /// The kind of the counter.
    pub kind: InfractionKind,
    /// The label key.
    pub label: &'static str,
    /// The count in the chosen set, sold history included.
    pub shown: u32,
    /// Served plus unserved of this kind.
    pub total: u32,
}

/// The eight rows, in counter index order.
pub fn infraction_rows(stable: &Stable, view: InfractionView) -> [InfractionRow; 8] {
    let shown = stable.infraction_totals(view.set());
    let unserved = stable.infraction_totals(InfractionSet::Unserved);
    let served = stable.infraction_totals(InfractionSet::Served);
    InfractionKind::ALL.map(|kind| InfractionRow {
        kind,
        label: INFRACTION_ROW_LABELS[kind.index()],
        shown: shown[kind.index()],
        total: unserved[kind.index()] + served[kind.index()],
    })
}

/// The status text of a vehicle database row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveStatus {
    /// Impound state not 0 (`RAP_VD_IMPOUNDED`).
    Impounded,
    /// Busted count not 0 (`RAP_VD_CONFIRMED_TO_DRIVE`).
    ConfirmedToDrive,
    /// Otherwise (`RAP_VD_SUSPECTED_TO_DRIVE`).
    SuspectedToDrive,
}

/// One row of the vehicle database page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleRow {
    /// The car handle.
    pub handle: u32,
    /// The front-end key of the car (its name label is looked up from the car type).
    pub frontend_key: u32,
    /// The car bounty (no sold part).
    pub bounty: u32,
    /// `fine(unserved counters)` of the car.
    pub outstanding_fines: u32,
    /// Sum of the eight unserved counters.
    pub unserved: u32,
    /// The status.
    pub status: DriveStatus,
    /// Times evaded.
    pub evaded: u16,
    /// Times busted.
    pub busted: u16,
}

fn status_of(record: &CareerRecord) -> DriveStatus {
    if !record.is_free_to_drive() {
        return DriveStatus::Impounded;
    }
    match record.busted {
        0 => DriveStatus::SuspectedToDrive,
        _ => DriveStatus::ConfirmedToDrive,
    }
}

/// The vehicle database rows: one per listed car in car slot order 0 to 199 (not by name or bounty). A car is
/// listed when it has a handle, a category bit, the owned bit and a used career record. Cars that were sold are
/// not listed; the page shows two rows at a time and clears the rest.
pub fn vehicle_rows(stable: &Stable) -> Vec<VehicleRow> {
    let mut rows = Vec::new();
    for car in stable.cars.iter().filter(|car| car.counts()) {
        let Some(record) = car.career_record_index().and_then(|i| stable.careers.get(i)).filter(|r| !r.is_unused())
        else {
            continue;
        };
        rows.push(VehicleRow {
            handle: car.handle,
            frontend_key: car.frontend_key,
            bounty: record.bounty,
            outstanding_fines: fine_of_counters(&record.unserved) as u32,
            unserved: record.unserved_total(),
            status: status_of(record),
            evaded: record.evaded,
            busted: record.busted,
        });
    }
    rows
}

/// One row of the cost to state page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostRow {
    /// The category label key.
    pub label: &'static str,
    /// The quantity.
    pub quantity: u32,
    /// The cost: quantity times the unit price, except the first row whose cost is the stored cash value.
    pub cost: u32,
}

/// The nine rows in page order. The lines are lifetime sums over non-busted pursuits and are **not** added up: the
/// header total is statistic 7, which also holds the helicopter spike strip term (extra 3, unit price 225, no
/// line) and may differ from the sum of the nine costs.
pub fn cost_rows(history: &PursuitHistory) -> [CostRow; 9] {
    let e = &history.extras;
    let priced = |label, quantity: u32, price: u32| CostRow { label, quantity, cost: quantity.saturating_mul(price) };
    [
        CostRow { label: "RAP_CTS_DAMAGE_TO_PROPERTY", quantity: e[7], cost: e[6] },
        priced("RAP_CTS_INSURANCE_CLAIMS", e[1], 500),
        priced("RAP_CTS_OFFICERS_INVOLVED", e[4], 250),
        priced("RAP_CTS_SUPPORT_VEHICLES_INVOLVED", e[5], 450),
        priced("RAP_CTS_DAMAGED_POLICE_VEHICLES", history.sums[2], 250),
        priced("RAP_CTS_DESTROYED_POLICE_VEHICLES", history.sums[3], 5000),
        priced("RAP_CTS_ROADBLOCKS_DEPLOYED", e[0], 500),
        priced("RAP_CTS_SPIKE_STRIPS_DELOYED", e[2], 250),
        priced("RAP_CTS_HELICOPTERS_INVOLVED", history.sums[6], 2000),
    ]
}

/// A direction of the up and down keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Up.
    Up,
    /// Down.
    Down,
}

/// The used flags of the five top pursuit entries.
pub fn top_used(history: &PursuitHistory) -> [bool; TOP_ENTRIES] {
    history.top.map(|entry: PursuitEntry| entry.is_used())
}

/// Up and down on the top 5 page wrap among the used entries only: up on the first used goes to the last used,
/// down on the last used goes to the first. `current` is the entry number 1 to 5. With no used entry, or a current
/// entry that is not used, the result is `None` (nothing to move to).
pub fn top_step(used: &[bool; TOP_ENTRIES], current: usize, step: Step) -> Option<usize> {
    let numbers: Vec<usize> = (1..=TOP_ENTRIES).filter(|&n| used[n - 1]).collect();
    let at = numbers.iter().position(|&n| n == current)?;
    let len = numbers.len();
    let next = match step {
        Step::Up => (at + len - 1) % len,
        Step::Down => (at + 1) % len,
    };
    Some(numbers[next])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stable::CarRecord;

    fn stable_with(records: &[(u8, CareerRecord)]) -> Stable {
        let mut s = Stable::empty();
        for (slot, (index, record)) in records.iter().enumerate() {
            s.cars[slot] = CarRecord {
                handle: 100 + slot as u32,
                frontend_key: 0xF000 + slot as u32,
                flags: 0xF_0002,
                career_index: *index,
                ..CarRecord::free()
            };
            s.careers[usize::from(*index)] = *record;
        }
        s
    }

    #[test]
    fn the_summary_index_follows_the_install_array() {
        let t = &SUMMARY_THRESHOLDS;
        let cases = [
            (0, 1),
            (1, 1),
            (100, 1),
            (101, 2),
            (400, 2),
            (401, 3),
            (800, 3),
            (801, 4),
            (1000, 4),
            (1001, 5),
            (u32::MAX, 5),
        ];
        for (total, nn) in cases {
            assert_eq!(summary_index(total, t), nn, "total {total}");
        }
        assert_eq!(summary_label(1), "RAP_SUMMARY_01");
        assert_eq!(summary_label(5), "RAP_SUMMARY_05");
    }

    #[test]
    fn the_summary_index_handles_other_arrays() {
        assert_eq!(summary_index(0, &[]), 1, "no thresholds: N is 0 and 0 prints as 1");
        assert_eq!(summary_index(5, &[10]), 1);
        assert_eq!(summary_index(50, &[10]), 1);
        assert_eq!(summary_index(5, &[1, 10]), 1);
        assert_eq!(summary_index(5, &[2, 4]), 2);
    }

    #[test]
    fn totals_count_the_live_cars_and_the_sold_history_as_the_spec_says() {
        let a = CareerRecord {
            bounty: 1000,
            evaded: 3,
            busted: 1,
            unserved: [1, 0, 0, 0, 0, 0, 0, 0],
            ..CareerRecord::new(0)
        };
        let mut b = CareerRecord { bounty: 250, evaded: 2, served: [0, 0, 2, 0, 0, 0, 0, 0], ..CareerRecord::new(1) };
        b.enter_impound(crate::stable::ImpoundState::LimitReached);
        let mut s = stable_with(&[(0, a), (1, b)]);
        s.sold.bounty = 5000;
        s.sold.unserved[1] = 7;
        s.sold.evaded = 10;
        let mut h = PursuitHistory::default();
        h.sums[7] = 123_456;
        h.sums[2] = 9;
        h.sums[3] = 4;
        let t = RapTotals::compute(&s, &h);
        assert_eq!(t.bounty, 6250);
        assert_eq!((t.cost_to_state, t.police_damaged, t.police_destroyed), (123_456, 9, 4));
        assert_eq!((t.cars_monitored, t.cars_impounded), (2, 1));
        assert_eq!(t.fines_due, 150, "sold unserved infractions have no part in the fines");
        assert_eq!(t.fines_paid, 2000);
        assert_eq!((t.times_evaded, t.times_busted), (15, 1));
        assert_eq!((t.unserved, t.served), (8, 2));
        assert_eq!(t.all_infractions(), 10);
        assert_eq!(t.verdict(), 1);
    }

    #[test]
    fn the_statistics_accessor_answers_0_to_9() {
        let a = CareerRecord { bounty: 40, unserved: [2; 8], served: [1; 8], ..CareerRecord::new(0) };
        let s = stable_with(&[(0, a)]);
        let h = PursuitHistory { sums: [10, 11, 12, 13, 14, 15, 16, 17], ..PursuitHistory::default() };
        assert_eq!(statistic(&s, &h, 0), Some(10));
        assert_eq!(statistic(&s, &h, 7), Some(17));
        assert_eq!(statistic(&s, &h, 8), Some(24));
        assert_eq!(statistic(&s, &h, 9), Some(40));
        assert_eq!(statistic(&s, &h, 10), None);
    }

    #[test]
    fn infraction_rows_show_the_chosen_set_and_the_combined_total() {
        let a = CareerRecord {
            unserved: [3, 0, 0, 0, 0, 0, 0, 1],
            served: [1, 0, 0, 0, 0, 0, 0, 0],
            ..CareerRecord::new(0)
        };
        let mut s = stable_with(&[(0, a)]);
        s.sold.unserved[0] = 5;
        let unserved = infraction_rows(&s, InfractionView::Unserved);
        assert_eq!((unserved[0].shown, unserved[0].total), (8, 9));
        assert_eq!(unserved[0].label, "RAP_US_RACING");
        assert_eq!(unserved[0].kind, InfractionKind::Speeding);
        assert_eq!((unserved[7].shown, unserved[7].total), (1, 1));
        let served = infraction_rows(&s, InfractionView::Served);
        assert_eq!((served[0].shown, served[0].total), (1, 9));
        assert_eq!(InfractionView::Unserved.toggled(), InfractionView::Served);
        assert_eq!(InfractionView::Served.toggled().toggled(), InfractionView::Served);
    }

    #[test]
    fn the_infraction_labels_keep_the_swapped_speeding_names() {
        assert_eq!(INFRACTION_ROW_LABELS[0], "RAP_US_RACING");
        assert_eq!(INFRACTION_ROW_LABELS[1], "RAP_US_EXCESSIVE_SPEEDING");
    }

    #[test]
    fn vehicle_rows_are_in_slot_order_with_the_status_rules() {
        let free = CareerRecord { bounty: 10, unserved: [0, 0, 1, 0, 0, 0, 0, 0], ..CareerRecord::new(0) };
        let busted = CareerRecord { busted: 2, evaded: 1, ..CareerRecord::new(1) };
        let mut impounded = CareerRecord { busted: 5, ..CareerRecord::new(2) };
        impounded.enter_impound(crate::stable::ImpoundState::CashConfiscated);
        let s = stable_with(&[(2, impounded), (0, free), (1, busted)]);
        let rows = vehicle_rows(&s);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].handle, 100);
        assert_eq!(rows[0].status, DriveStatus::Impounded, "an impound wins over the busted count");
        assert_eq!(rows[1].status, DriveStatus::SuspectedToDrive);
        assert_eq!((rows[1].bounty, rows[1].outstanding_fines, rows[1].unserved), (10, 1000, 1));
        assert_eq!(rows[2].status, DriveStatus::ConfirmedToDrive);
        assert_eq!((rows[2].evaded, rows[2].busted), (1, 2));
        assert_eq!(rows[1].frontend_key, 0xF001);
    }

    #[test]
    fn unlisted_cars_do_not_make_rows() {
        let mut s = stable_with(&[(0, CareerRecord::new(0))]);
        s.cars[1] = CarRecord { handle: 7, flags: 0x2, career_index: 0, ..CarRecord::free() };
        s.cars[2] = CarRecord { handle: 8, flags: 0xF_0002, career_index: 9, ..CarRecord::free() };
        assert_eq!(vehicle_rows(&s).len(), 1);
    }

    #[test]
    fn cost_rows_use_the_unit_prices_of_the_page() {
        let mut h = PursuitHistory { extras: [10, 20, 30, 40, 50, 60, 7_000, 8], ..PursuitHistory::default() };
        h.sums[2] = 3;
        h.sums[3] = 2;
        h.sums[6] = 5;
        let rows = cost_rows(&h);
        let table: Vec<(&str, u32, u32)> = rows.iter().map(|r| (r.label, r.quantity, r.cost)).collect();
        assert_eq!(
            table,
            [
                ("RAP_CTS_DAMAGE_TO_PROPERTY", 8, 7_000),
                ("RAP_CTS_INSURANCE_CLAIMS", 20, 10_000),
                ("RAP_CTS_OFFICERS_INVOLVED", 50, 12_500),
                ("RAP_CTS_SUPPORT_VEHICLES_INVOLVED", 60, 27_000),
                ("RAP_CTS_DAMAGED_POLICE_VEHICLES", 3, 750),
                ("RAP_CTS_DESTROYED_POLICE_VEHICLES", 2, 10_000),
                ("RAP_CTS_ROADBLOCKS_DEPLOYED", 10, 5_000),
                ("RAP_CTS_SPIKE_STRIPS_DELOYED", 30, 7_500),
                ("RAP_CTS_HELICOPTERS_INVOLVED", 5, 10_000),
            ]
        );
    }

    #[test]
    fn the_cost_page_does_not_add_the_lines_up() {
        let mut h = PursuitHistory::default();
        h.sums[7] = 1;
        h.extras[3] = 100;
        assert_eq!(RapTotals::compute(&Stable::empty(), &h).cost_to_state, 1);
        assert!(cost_rows(&h).iter().all(|r| r.cost == 0), "helicopter spike strips (extra 3) have no line");
    }

    #[test]
    fn top_navigation_wraps_among_the_used_entries() {
        let used = [true, true, false, true, false];
        assert_eq!(top_step(&used, 1, Step::Up), Some(4));
        assert_eq!(top_step(&used, 4, Step::Down), Some(1));
        assert_eq!(top_step(&used, 1, Step::Down), Some(2));
        assert_eq!(top_step(&used, 2, Step::Down), Some(4));
        assert_eq!(top_step(&used, 4, Step::Up), Some(2));
        assert_eq!(top_step(&used, 3, Step::Down), None, "an unused entry cannot be current");
        assert_eq!(top_step(&[false; 5], 1, Step::Down), None);
        assert_eq!(top_step(&[true, false, false, false, false], 1, Step::Down), Some(1));
    }

    #[test]
    fn top_used_follows_the_duration_field() {
        let mut h = PursuitHistory::default();
        h.top[0].ticks = 4000;
        h.top[2].ticks = 1;
        assert_eq!(top_used(&h), [true, false, true, false, false]);
    }
}
