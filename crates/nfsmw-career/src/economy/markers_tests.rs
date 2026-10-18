use super::*;

/// A table with a recognisable value in every byte.
fn synthetic_table() -> Vec<u8> {
    (0..POPUP_QUEUE_LEN).map(|i| (i % 251) as u8).collect()
}

#[test]
fn the_table_has_63_entries_of_12_bytes() {
    assert_eq!(ENTRY_LEN, 12);
    assert_eq!(ENTRY_COUNT, 63);
    assert_eq!(MarkerInventory::empty().to_table().len(), POPUP_QUEUE_LEN);
}

#[test]
fn every_byte_of_the_table_round_trips() {
    let table = synthetic_table();
    let inventory = MarkerInventory::from_table(&table).unwrap();
    assert_eq!(inventory.to_table(), table);
}

#[test]
fn unknown_kinds_and_flags_are_kept_and_not_counted() {
    let mut table = vec![0u8; POPUP_QUEUE_LEN];
    // Entry 0: a notice of type 0x12 with flag 7. Entry 1: a jail-free marker with flag 0 (free slot).
    table[0..4].copy_from_slice(&0x12u32.to_le_bytes());
    table[8..12].copy_from_slice(&7u32.to_le_bytes());
    table[12..16].copy_from_slice(&0x11u32.to_le_bytes());
    let mut inventory = MarkerInventory::from_table(&table).unwrap();
    assert_eq!(inventory.count(MarkerKind::JailFree), 0, "a free slot is not a marker");
    assert_eq!(inventory.to_table(), table, "the notice survives untouched");
    assert_eq!(inventory.add(MarkerKind::JailFree, 1), 1);
    assert_eq!(&inventory.to_table()[0..12], &table[0..12], "the first notice is not overwritten");
    assert_eq!(&inventory.to_table()[12..16], &0x11u32.to_le_bytes(), "the marker goes in the first free slot");
}

#[test]
fn a_short_table_is_refused() {
    assert!(MarkerInventory::from_table(&[0u8; POPUP_QUEUE_LEN - 1]).is_err());
}

#[test]
fn a_new_inventory_holds_no_markers() {
    let inventory = MarkerInventory::empty();
    assert_eq!(inventory.count(MarkerKind::JailFree), 0);
    assert_eq!(inventory.count(MarkerKind::ExtraStrike), 0);
    assert_eq!(inventory.count(MarkerKind::ImpoundRelease), 0);
}

#[test]
fn add_counts_each_kind_separately_and_writes_flag_one() {
    let mut inventory = MarkerInventory::empty();
    assert_eq!(inventory.add(MarkerKind::JailFree, 4), 4);
    assert_eq!(inventory.add(MarkerKind::ExtraStrike, 1), 1);
    assert_eq!(inventory.count(MarkerKind::JailFree), 4);
    assert_eq!(inventory.count(MarkerKind::ExtraStrike), 1);
    assert_eq!(inventory.count(MarkerKind::ImpoundRelease), 0);
    let table = inventory.to_table();
    assert_eq!(u32_at(&table, 0), 0x11);
    assert_eq!(u32_at(&table, 8), 1);
    assert_eq!(u32_at(&table, 4 * ENTRY_LEN), 0x14, "the fifth entry is the first free one after the four");
}

#[test]
fn add_stops_when_the_table_is_full() {
    let mut inventory = MarkerInventory::empty();
    assert_eq!(inventory.add(MarkerKind::JailFree, ENTRY_COUNT as u32), ENTRY_COUNT as u32);
    assert_eq!(inventory.add(MarkerKind::JailFree, 2), 0);
    assert_eq!(inventory.count(MarkerKind::JailFree), ENTRY_COUNT as u32);
}

#[test]
fn take_removes_one_marker_and_reports_when_none_is_held() {
    let mut inventory = MarkerInventory::empty();
    inventory.add(MarkerKind::JailFree, 2);
    assert!(inventory.take(MarkerKind::JailFree));
    assert_eq!(inventory.count(MarkerKind::JailFree), 1);
    assert!(inventory.take(MarkerKind::JailFree));
    assert_eq!(inventory.count(MarkerKind::JailFree), 0);
    assert!(!inventory.take(MarkerKind::JailFree), "none left to take");
    assert_eq!(inventory, MarkerInventory::empty(), "a taken slot is free again");
}

#[test]
fn take_leaves_other_kinds_alone() {
    let mut inventory = MarkerInventory::empty();
    inventory.add(MarkerKind::ImpoundRelease, 1);
    assert!(!inventory.take(MarkerKind::JailFree));
    assert_eq!(inventory.count(MarkerKind::ImpoundRelease), 1);
}

#[test]
fn the_kinds_have_the_documented_types() {
    assert_eq!(MarkerKind::JailFree.code(), 0x11);
    assert_eq!(MarkerKind::ExtraStrike.code(), 0x14);
    assert_eq!(MarkerKind::ImpoundRelease.code(), 0x15);
}
