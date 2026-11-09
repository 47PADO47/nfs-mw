use super::*;
use crate::build_pieces::Mounts;
use crate::options::{BikeOptions, Exhaust, Pair};
use crate::partfile::Anchor;

fn anchor() -> Anchor {
    Anchor::rigid(None, [0.0; 3], [0.0, 0.0, 0.0, 1.0])
}

fn plan(options: &BikeOptions) -> Plan {
    let have: Vec<_> = PIECES.iter().collect();
    Plan::of("ktm450", options, &have, Mounts::default())
}

fn exhaust() -> Exhaust {
    Exhaust {
        id: "MUF_A_B".to_owned(),
        header: "h".to_owned(),
        muffler: "m".to_owned(),
        header_anchor: anchor(),
        muffler_anchor: anchor(),
        muffler_twins: Vec::new(),
    }
}

#[test]
fn the_stock_parts_a_plan_binds_are_the_ones_that_become_solids() {
    let only_exhaust = plan(&BikeOptions { exhaust: vec![exhaust()], ..BikeOptions::default() });
    assert_eq!(split_mask(&only_exhaust), 0b1100_0001, "the road equipment is split whatever the catalogue offers");
    let rims = Pair { id: "RI_X_Y".to_owned(), level: 0, asset: "rim1".to_owned(), front: anchor(), back: anchor() };
    let both = plan(&BikeOptions { exhaust: vec![exhaust()], rims: vec![rims], ..BikeOptions::default() });
    assert_eq!(
        split_mask(&both),
        0b1100_1101,
        "the exhaust, the two rims and the road equipment; the bar pad and the discs stay in their solids"
    );
    assert_eq!(split_mask(&Plan::default()), 0);
}

#[test]
fn the_plans_of_the_bikes_are_kept_for_the_catalog_file() {
    let catalogue = Catalogue::default();
    let plans = Mutex::new(Vec::new());
    let shop = Shop { catalogue: &catalogue, plans: &plans, library: None };
    shop.keep(plan(&BikeOptions { exhaust: vec![exhaust()], ..BikeOptions::default() }));
    assert_eq!(plans.lock().unwrap().len(), 1);
}
