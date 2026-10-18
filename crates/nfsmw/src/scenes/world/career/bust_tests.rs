use super::*;
use nfsmw_career::settings::Flags;

fn settings_with_cash(cash: u32) -> CareerSettings {
    CareerSettings { cash, ..CareerSettings::new_career() }
}

/// A record that owes one speeding ticket (150) and has a busted count of 1.
fn record_owing_speeding() -> CareerRecord {
    let mut record = CareerRecord::new(0);
    record.unserved[InfractionKind::Speeding.index()] = 1;
    record.busted = 1;
    record.heat = 4.0;
    record
}

#[test]
fn the_bust_infractions_map_to_the_career_kinds_in_order() {
    let mapped: Vec<InfractionKind> = Infraction::ALL.into_iter().map(kind_of).collect();
    assert_eq!(mapped, InfractionKind::ALL);
}

#[test]
fn the_pursuit_mask_holds_each_infraction_once() {
    let mask = pursuit_mask(&[Infraction::OffRoad, Infraction::Speeding, Infraction::OffRoad]);
    assert_eq!(mask, InfractionMask::of(&[InfractionKind::Speeding, InfractionKind::OffRoad]));
    assert_eq!(pursuit_mask(&[]), InfractionMask::NONE);
}

#[test]
fn cash_pays_the_unserved_fines_and_takes_a_strike() {
    let mut settings = settings_with_cash(5000);
    let mut record = record_owing_speeding();
    let pursuit = pursuit_mask(&[Infraction::Speeding]);
    let mut markers = MarkerInventory::empty();
    let outcome = settle_payment(&mut settings, &mut record, &mut markers, pursuit, false);
    assert_eq!(outcome, Some(PayOutcome::Paid));
    assert_eq!(settings.cash, 4850);
    assert_eq!((record.strikes, record.heat, record.state), (1, 1.0, 0));
    assert_eq!(record.unserved_total(), 0);
    assert_eq!(record.served[InfractionKind::Speeding.index()], 1);
}

#[test]
fn a_fine_above_the_cash_takes_the_cash_and_impounds_the_car() {
    let mut settings = settings_with_cash(100);
    let mut record = record_owing_speeding();
    let pursuit = pursuit_mask(&[Infraction::Speeding]);
    let mut markers = MarkerInventory::empty();
    let outcome = settle_payment(&mut settings, &mut record, &mut markers, pursuit, false);
    assert_eq!(outcome, Some(PayOutcome::InsufficientFunds));
    assert_eq!(settings.cash, 0);
    assert_eq!((record.state, record.strikes, record.countdown), (3, 3, 5));
}

#[test]
fn a_marker_takes_only_this_pursuit_and_leaves_cash_and_heat() {
    let mut settings = settings_with_cash(300);
    let mut record = record_owing_speeding();
    record.unserved[InfractionKind::OffRoad.index()] = 2;
    let pursuit = pursuit_mask(&[Infraction::Speeding]);
    let mut markers = MarkerInventory::empty();
    markers.add(MarkerKind::JailFree, 1);
    let outcome = settle_payment(&mut settings, &mut record, &mut markers, pursuit, true);
    assert_eq!(outcome, None);
    assert_eq!(markers.count(MarkerKind::JailFree), 0, "the marker is used up");
    assert_eq!(settings.cash, 300);
    assert_eq!(record.unserved[InfractionKind::Speeding.index()], 0);
    assert_eq!(record.unserved[InfractionKind::OffRoad.index()], 2, "the older infractions stay");
    assert_eq!((record.busted, record.strikes, record.heat, record.state), (0, 0, 4.0, 0));
}

#[test]
fn the_first_visit_grants_four_markers_to_a_player_with_none() {
    let mut settings = settings_with_cash(5000);
    let mut record = record_owing_speeding();
    let mut markers = MarkerInventory::empty();
    let pursuit = pursuit_mask(&[Infraction::Speeding]);
    settle_payment(&mut settings, &mut record, &mut markers, pursuit, false);
    assert_eq!(markers.count(MarkerKind::JailFree), 4);
}

#[test]
fn a_later_visit_grants_no_markers() {
    let mut settings = settings_with_cash(5000);
    settings.flags.insert(Flags::INFRACTIONS_SEEN);
    let mut record = record_owing_speeding();
    let mut markers = MarkerInventory::empty();
    let pursuit = pursuit_mask(&[Infraction::Speeding]);
    settle_payment(&mut settings, &mut record, &mut markers, pursuit, false);
    assert_eq!(markers.count(MarkerKind::JailFree), 0);
}

#[test]
fn the_first_visit_grants_nothing_to_a_player_already_holding_one() {
    let mut settings = settings_with_cash(5000);
    let mut record = record_owing_speeding();
    let mut markers = MarkerInventory::empty();
    markers.add(MarkerKind::JailFree, 1);
    let pursuit = pursuit_mask(&[Infraction::Speeding]);
    settle_payment(&mut settings, &mut record, &mut markers, pursuit, false);
    assert_eq!(markers.count(MarkerKind::JailFree), 1);
}

#[test]
fn a_marker_payment_on_the_first_visit_uses_a_held_marker_and_grants_nothing() {
    let mut settings = settings_with_cash(300);
    let mut record = record_owing_speeding();
    let mut markers = MarkerInventory::empty();
    markers.add(MarkerKind::JailFree, 1);
    let pursuit = pursuit_mask(&[Infraction::Speeding]);
    settle_payment(&mut settings, &mut record, &mut markers, pursuit, true);
    assert_eq!(markers.count(MarkerKind::JailFree), 0);
}
