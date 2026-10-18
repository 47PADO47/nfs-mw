use nfsmw_career::rapsheet::{RapTotals, summary_label};

fn totals(infractions: u32) -> RapTotals {
    RapTotals {
        bounty: 0,
        cost_to_state: 0,
        cars_impounded: 0,
        cars_monitored: 0,
        fines_due: 0,
        fines_paid: 0,
        times_evaded: 0,
        times_busted: 0,
        unserved: infractions,
        served: 0,
        police_damaged: 0,
        police_destroyed: 0,
    }
}

#[test]
fn the_verdict_line_follows_the_infraction_count() {
    assert_eq!(summary_label(totals(0).verdict()), "RAP_SUMMARY_01");
    assert_eq!(summary_label(totals(150).verdict()), "RAP_SUMMARY_02");
    assert_eq!(summary_label(totals(5_000).verdict()), "RAP_SUMMARY_05");
}
