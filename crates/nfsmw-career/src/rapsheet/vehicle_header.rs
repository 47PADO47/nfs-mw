//! The two cars the vehicle database names in its header (`docs/specs/career-rapsheet.md` section 6): the preferred
//! vehicle, the one with the most pursuits, and the recent vehicle, the one of the last pursuit.

use super::history::PursuitHistory;
use super::pages::VehicleRow;

/// The preferred vehicle: the listed car with the most pursuits (times evaded plus times busted). The first car in slot
/// order wins a tie, and a car with no pursuit never counts, so a stable with none has no preferred vehicle. **[exe]**
pub fn preferred_vehicle(rows: &[VehicleRow]) -> Option<&VehicleRow> {
    let mut best: Option<&VehicleRow> = None;
    let mut most = 0u32;
    for row in rows {
        let pursuits = u32::from(row.evaded) + u32::from(row.busted);
        if pursuits <= most {
            continue;
        }
        most = pursuits;
        best = Some(row);
    }
    best
}

/// The front-end key of the recent vehicle: the car of the last pursuit, busted or not. `None` before the first
/// pursuit (the history holds key 0). **[exe]**
pub fn recent_vehicle_key(history: &PursuitHistory) -> Option<u32> {
    Some(history.last_car_key).filter(|&key| key != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rapsheet::pages::DriveStatus;

    fn row(key: u32, evaded: u16, busted: u16) -> VehicleRow {
        VehicleRow {
            handle: key,
            frontend_key: key,
            bounty: 0,
            outstanding_fines: 0,
            unserved: 0,
            status: DriveStatus::SuspectedToDrive,
            evaded,
            busted,
        }
    }

    #[test]
    fn the_preferred_vehicle_has_the_most_pursuits() {
        let rows = [row(1, 1, 0), row(2, 2, 3), row(3, 4, 0)];
        assert_eq!(preferred_vehicle(&rows).map(|r| r.frontend_key), Some(2));
    }

    #[test]
    fn a_tie_goes_to_the_first_car_and_no_pursuits_means_no_preferred_vehicle() {
        let tie = [row(1, 2, 0), row(2, 0, 2)];
        assert_eq!(preferred_vehicle(&tie).map(|r| r.frontend_key), Some(1));
        assert_eq!(preferred_vehicle(&[row(1, 0, 0)]), None);
        assert_eq!(preferred_vehicle(&[]), None);
    }

    #[test]
    fn the_recent_vehicle_is_the_car_of_the_last_pursuit() {
        let mut history = PursuitHistory::default();
        assert_eq!(recent_vehicle_key(&history), None);
        history.last_car_key = 0xF001;
        assert_eq!(recent_vehicle_key(&history), Some(0xF001));
    }
}
