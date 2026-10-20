use super::*;
use crate::economy::{InfractionKind, InfractionMask, InfractionsScreen};

fn settings(cash: u32) -> CareerSettings {
    CareerSettings { cash, ..CareerSettings::new_career() }
}

fn with_markers(kind: MarkerKind, n: u32) -> MarkerInventory {
    let mut markers = MarkerInventory::empty();
    markers.add(kind, n);
    markers
}

fn releasable() -> CareerRecord {
    let mut record = CareerRecord::new(0);
    record.enter_impound(ImpoundState::LimitReached);
    record.countdown = 1;
    record.win_tick();
    record
}

#[test]
fn entry_arguments_are_the_ones_the_original_queues() {
    assert_eq!(CarSelectEntry::Plain.arg(), 0);
    assert_eq!(CarSelectEntry::NewStrike.arg(), 0x100);
    assert_eq!(CarSelectEntry::Impounded.arg(), 0x200);
}

#[test]
fn a_marker_payment_goes_straight_to_the_car_select_with_no_animation() {
    let after = after_payment(Payment::Marker, None);
    assert_eq!(after, BustAftermath { notice: None, entry: CarSelectEntry::Plain });
}

#[test]
fn a_cash_payment_that_keeps_the_car_plays_the_new_strike() {
    let after = after_payment(Payment::Cash, Some(PayOutcome::Paid));
    assert_eq!(after, BustAftermath { notice: None, entry: CarSelectEntry::NewStrike });
}

#[test]
fn an_impounding_payment_shows_its_notice_before_the_impounded_stamp() {
    let limit = after_payment(Payment::Cash, Some(PayOutcome::StrikeLimit));
    assert_eq!(limit, BustAftermath { notice: Some(BustNotice::StrikeLimit), entry: CarSelectEntry::Impounded });
    let poor = after_payment(Payment::Cash, Some(PayOutcome::InsufficientFunds));
    assert_eq!(poor.notice, Some(BustNotice::InsufficientFunds));
    assert_eq!(poor.entry, CarSelectEntry::Impounded);
}

#[test]
fn the_aftermath_is_read_back_from_the_record_the_payment_left() {
    let mut record = CareerRecord::new(0);
    assert_eq!(after_payment_of(Payment::Cash, Some(&record)).entry, CarSelectEntry::NewStrike);
    record.enter_impound(ImpoundState::CashConfiscated);
    let after = after_payment_of(Payment::Cash, Some(&record));
    assert_eq!(after.notice, Some(BustNotice::InsufficientFunds));
    assert_eq!(after_payment_of(Payment::Marker, Some(&record)).notice, None, "a marker never impounds");
    assert_eq!(after_payment_of(Payment::Cash, None).entry, CarSelectEntry::NewStrike);
}

#[test]
fn the_aftermath_of_a_real_third_bust_is_the_strike_limit_notice() {
    let mut s = settings(100_000);
    let mut record = CareerRecord::new(0);
    record.strikes = 2;
    record.add_unserved(InfractionMask::of(&[InfractionKind::Speeding]));
    let screen = InfractionsScreen::open(&mut s, &record, InfractionMask::NONE, false);
    let outcome = screen.pay_with_cash(&mut s, &mut record);
    let after = after_payment(Payment::Cash, Some(outcome));
    assert_eq!(after.notice, Some(BustNotice::StrikeLimit));
    assert_eq!(after_payment_of(Payment::Cash, Some(&record)), after);
}

#[test]
fn notice_labels_are_the_original_strings_table_keys() {
    assert_eq!(BustNotice::StrikeLimit.label(), "MW_BUSTED_STRIKE_LIMIT");
    assert_eq!(BustNotice::InsufficientFunds.label(), "MW_BUSTED_INSUFFICIENT_FUNDS");
}

#[test]
fn the_car_select_numbers_follow_the_record() {
    let mut record = CareerRecord::new(0);
    record.strikes = 2;
    record.heat = 3.5;
    record.bounty = 1200;
    record.unserved[0] = 2;
    let info = car_select_info(&record, 35_000);
    assert_eq!(info.status, CarStatus::Free);
    assert_eq!((info.strikes, info.strike_limit), (2, 3));
    assert_eq!((info.bounty, info.fines, info.release_cost), (1200, 300, 8_750));
    record.enter_impound(ImpoundState::CashConfiscated);
    assert_eq!(car_select_info(&record, 35_000).status, CarStatus::Impounded);
    assert_eq!(car_select_info(&releasable(), 1).status, CarStatus::Releasable);
}

#[test]
fn strike_pips_never_exceed_five() {
    let mut record = CareerRecord::new(0);
    record.strike_limit = 9;
    record.strikes = 9;
    let info = car_select_info(&record, 0);
    assert_eq!((info.strikes, info.strike_limit), (5, 5));
}

#[test]
fn back_is_refused_while_the_original_car_is_impounded() {
    assert!(!back_is_blocked(&CareerRecord::new(0)));
    let mut record = CareerRecord::new(0);
    record.enter_impound(ImpoundState::LimitReached);
    assert!(back_is_blocked(&record));
    assert!(back_is_blocked(&releasable()));
}

#[test]
fn the_release_dialog_offers_cash_only_for_a_releasable_car_that_can_be_paid() {
    let none = MarkerInventory::empty();
    let record = releasable();
    assert_eq!(
        release_dialog(&record, &settings(8_750), 35_000, &none),
        ReleaseDialog::Choose { cash: true, marker: false }
    );
    assert_eq!(release_dialog(&record, &settings(8_749), 35_000, &none), ReleaseDialog::Cannot);
    let mut counting = CareerRecord::new(0);
    counting.enter_impound(ImpoundState::LimitReached);
    assert_eq!(release_dialog(&counting, &settings(1_000_000), 35_000, &none), ReleaseDialog::Cannot);
}

#[test]
fn a_release_marker_is_offered_while_the_car_still_counts_down() {
    let markers = with_markers(MarkerKind::ImpoundRelease, 1);
    let mut counting = CareerRecord::new(0);
    counting.enter_impound(ImpoundState::CashConfiscated);
    assert_eq!(
        release_dialog(&counting, &settings(0), 35_000, &markers),
        ReleaseDialog::Choose { cash: false, marker: true }
    );
    assert_eq!(
        release_dialog(&releasable(), &settings(1_000_000), 35_000, &markers),
        ReleaseDialog::Choose { cash: true, marker: true }
    );
}

#[test]
fn releasing_with_cash_pays_the_fee_and_clears_the_record() {
    let mut s = settings(10_000);
    let mut record = releasable();
    record.heat = 4.0;
    let mut markers = MarkerInventory::empty();
    assert_eq!(release(ReleaseWay::Cash, &mut s, &mut record, 35_000, &mut markers), Ok(()));
    assert_eq!(s.cash, 1_250);
    assert_eq!((record.state, record.strikes, record.heat), (0, 0, 0.0));
}

#[test]
fn releasing_with_a_marker_consumes_it_and_refuses_without_one() {
    let mut s = settings(0);
    let mut record = CareerRecord::new(0);
    record.enter_impound(ImpoundState::LimitReached);
    let mut markers = MarkerInventory::empty();
    assert_eq!(
        release(ReleaseWay::Marker, &mut s, &mut record, 35_000, &mut markers),
        Err(ReleaseError::NotReleasable)
    );
    assert_eq!(record.state, 2, "nothing changed");
    markers.add(MarkerKind::ImpoundRelease, 2);
    assert_eq!(release(ReleaseWay::Marker, &mut s, &mut record, 35_000, &mut markers), Ok(()));
    assert_eq!(record.state, 0);
    assert_eq!(markers.count(MarkerKind::ImpoundRelease), 1);
}

#[test]
fn releasing_with_cash_refuses_a_poor_player_and_keeps_the_car_impounded() {
    let mut s = settings(100);
    let mut record = releasable();
    let mut markers = MarkerInventory::empty();
    let err = release(ReleaseWay::Cash, &mut s, &mut record, 35_000, &mut markers).unwrap_err();
    assert_eq!(err, ReleaseError::InsufficientCash { fee: 8_750 });
    assert_eq!((s.cash, record.state), (100, 4));
}

#[test]
fn the_strike_marker_raises_a_free_cars_limit_to_five() {
    let mut markers = with_markers(MarkerKind::ExtraStrike, 3);
    let mut record = CareerRecord::new(0);
    assert_eq!(strike_marker(&record, &markers), StrikeMarker::Offer);
    assert!(use_strike_marker(&mut record, &mut markers));
    assert!(use_strike_marker(&mut record, &mut markers));
    assert_eq!(record.strike_limit, 5);
    assert_eq!(strike_marker(&record, &markers), StrikeMarker::AtMaximum);
    assert!(!use_strike_marker(&mut record, &mut markers), "at five it is refused and the marker kept");
    assert_eq!(markers.count(MarkerKind::ExtraStrike), 1);
}

#[test]
fn the_strike_marker_is_not_offered_for_an_impounded_car_or_without_one() {
    let markers = with_markers(MarkerKind::ExtraStrike, 1);
    let mut impounded = CareerRecord::new(0);
    impounded.enter_impound(ImpoundState::LimitReached);
    assert_eq!(strike_marker(&impounded, &markers), StrikeMarker::Nothing);
    assert_eq!(strike_marker(&CareerRecord::new(0), &MarkerInventory::empty()), StrikeMarker::Nothing);
    let mut markers = markers;
    assert!(!use_strike_marker(&mut impounded, &mut markers));
    assert_eq!(markers.count(MarkerKind::ExtraStrike), 1);
}
