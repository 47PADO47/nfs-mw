use super::*;

fn selection(car: &str) -> Selection {
    Selection { car: car.to_owned(), ..Selection::default() }
}

#[test]
fn every_garage_category_maps_to_its_own_cooling_part() {
    let mapped: Vec<CoolingPart> = Category::ALL.into_iter().map(part_of).collect();
    assert_eq!(
        mapped,
        vec![
            CoolingPart::BodyKit,
            CoolingPart::Spoiler,
            CoolingPart::Hood,
            CoolingPart::RoofScoop,
            CoolingPart::Rim,
            CoolingPart::Paint,
            CoolingPart::WindowTint,
            CoolingPart::RimPaint,
            CoolingPart::Vinyl
        ]
    );
}

#[test]
fn changed_slots_and_a_new_colour_are_the_fitted_parts() {
    let old = selection("m3gtr");
    let mut new = old.clone();
    new.parts.parts[Category::Spoiler as usize] = Some(7);
    new.parts.parts[Category::Wheels as usize] = Some(9);
    new.color = Some([1, 2, 3]);
    assert_eq!(fitted_parts(&old, &new), vec![CoolingPart::Spoiler, CoolingPart::Rim, CoolingPart::Paint]);
}

#[test]
fn an_unchanged_selection_fits_nothing() {
    let old = selection("m3gtr");
    assert!(fitted_parts(&old, &old.clone()).is_empty());
}

#[test]
fn decals_cool_per_slot_and_only_the_original_number_slot_cools() {
    let old = selection("CAR");
    let mut new = old.clone();
    new.parts.decals[2][0] = Some(1);
    new.parts.decals[2][1] = Some(2);
    new.parts.decals[2][6] = Some(3);
    new.parts.decals[2][7] = Some(4);
    new.parts.decals[3][6] = Some(3);
    new.parts.decals[3][7] = Some(4);
    assert_eq!(fitted_parts(&old, &new), vec![CoolingPart::Decal, CoolingPart::Decal, CoolingPart::BodyKit]);
    new.parts.vinyl_colors[0] = Some([1, 2, 3]);
    assert_eq!(fitted_parts(&old, &new).len(), 3, "auxiliary vinyl color has no cooling switch");
}

#[test]
fn a_car_swap_or_a_first_selection_is_not_a_purchase() {
    let mut new = selection("m3gtr");
    new.parts.parts[Category::Hood as usize] = Some(4);
    assert!(fitted_parts(&selection(""), &new).is_empty(), "nothing applied yet");
    assert!(fitted_parts(&selection("elise"), &new).is_empty(), "another car");
}

#[test]
fn going_back_to_the_stock_part_counts_as_fitting_a_part() {
    let mut old = selection("m3gtr");
    old.parts.parts[Category::Body as usize] = Some(3);
    let new = selection("m3gtr");
    assert_eq!(fitted_parts(&old, &new), vec![CoolingPart::BodyKit]);
}

/// The install's table reads, and the garage's parts cool a hot record. Skipped without `NFSMW_GAME_DIR`.
#[test]
#[ignore = "needs the installed game"]
fn the_installed_table_cools_a_hot_car_part_by_part() {
    let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = game_install::GameDir::open(dir).unwrap();
    let table = read_table(&dir).expect("the install has the fecooling table");
    let mut record = nfsmw_career::stable::CareerRecord::new(0);
    record.heat = 4.0;
    for part in [CoolingPart::Paint, CoolingPart::BodyKit] {
        record.cool_for_part(&table, part, SHOP_EXTRA);
    }
    assert!((record.heat - 4.0 * 0.7 * 0.69).abs() < 1e-4, "heat {}", record.heat);
}

#[test]
fn changing_paint_finish_and_rgb_cools_once_and_tint_is_independent() {
    let old = selection("m3gtr");
    let mut new = old.clone();
    new.parts.parts[Category::Paint as usize] = Some(100);
    new.parts.parts[Category::WindowTint as usize] = Some(200);
    new.color = Some([10, 20, 30]);
    assert_eq!(fitted_parts(&old, &new), vec![CoolingPart::Paint, CoolingPart::WindowTint]);
}
