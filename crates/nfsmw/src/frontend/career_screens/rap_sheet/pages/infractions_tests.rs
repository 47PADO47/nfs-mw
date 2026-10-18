use nfsmw_career::rapsheet::{InfractionView, RapTotals};

use super::*;

fn totals() -> RapTotals {
    RapTotals {
        bounty: 5_000,
        cost_to_state: 0,
        cars_impounded: 0,
        cars_monitored: 1,
        fines_due: 1_500,
        fines_paid: 40,
        times_evaded: 2,
        times_busted: 1,
        unserved: 8,
        served: 3,
        police_damaged: 0,
        police_destroyed: 0,
    }
}

#[test]
fn the_unserved_view_shows_the_unserved_count_and_the_fines_due() {
    let head = view_header(InfractionView::Unserved, &totals());
    assert_eq!(head.title, "RAP_UNSERVED_INFRACTIONS");
    assert_eq!((head.count, head.fines, head.fines_value), (8, "RAP_FINES_DUE", 1_500));
    assert_eq!(head.column, "RAP_US_UNSERVED");
}

#[test]
fn the_served_view_shows_the_served_count_and_the_fines_paid() {
    let head = view_header(InfractionView::Served, &totals());
    assert_eq!(head.title, "RAP_SERVED_INFRACTIONS");
    assert_eq!((head.count, head.fines, head.fines_value), (3, "RAP_FINES_PAID", 40));
    assert_eq!(head.column, "RAP_US_SERVED");
}
