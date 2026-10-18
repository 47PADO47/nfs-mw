use super::*;

/// A table with a recognisable value in every byte.
fn synthetic_table() -> Vec<u8> {
    (0..INVENTORY_LEN).map(|i| (i % 251) as u8).collect()
}

#[test]
fn the_table_has_63_entries_of_12_bytes() {
    assert_eq!(ENTRY_LEN, 12);
    assert_eq!(ENTRY_COUNT, 63);
    assert_eq!(MarkerInventory::empty().to_table().len(), INVENTORY_LEN);
}

#[test]
fn every_byte_of_the_table_round_trips() {
    let table = synthetic_table();
    let inventory = MarkerInventory::from_table(&table).unwrap();
    assert_eq!(inventory.to_table(), table);
}

#[test]
fn inactive_entries_and_unknown_kinds_are_kept_and_not_counted() {
    let mut table = vec![0u8; INVENTORY_LEN];
    // Entry 0: an unknown kind 0x12 with flag 7 (not active). Entry 1: a jail-free marker with flag 0 (not active,
    // and not free because its type is set).
    table[0..4].copy_from_slice(&0x12u32.to_le_bytes());
    table[8..12].copy_from_slice(&7u32.to_le_bytes());
    table[12..16].copy_from_slice(&0x11u32.to_le_bytes());
    let mut inventory = MarkerInventory::from_table(&table).unwrap();
    assert_eq!(inventory.count(MarkerKind::JailFree), 0, "an inactive entry is not a marker");
    assert_eq!(inventory.to_table(), table, "both entries survive untouched");
    assert_eq!(inventory.add(MarkerKind::JailFree, 1), 1);
    assert_eq!(&inventory.to_table()[0..24], &table[0..24], "neither entry is overwritten");
    assert_eq!(&inventory.to_table()[24..28], &0x11u32.to_le_bytes(), "the marker goes in the first type-0 slot");
    assert_eq!(&inventory.to_table()[28..32], &0u32.to_le_bytes());
    assert_eq!(&inventory.to_table()[32..36], &1u32.to_le_bytes());
}

#[test]
fn a_short_table_is_refused() {
    assert!(MarkerInventory::from_table(&[0u8; INVENTORY_LEN - 1]).is_err());
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
    assert_eq!(u32_at(&table, 4), 0, "aux is 0 for a marker");
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
fn take_keeps_the_aux_word_of_the_removed_entry() {
    let mut inventory = MarkerInventory::empty();
    assert_eq!(inventory.add_entries(0x2, 0x5, 1), 1);
    assert!(inventory.take_entry(0x2, 0x5));
    let table = inventory.to_table();
    assert_eq!(&table[0..12], &[0, 0, 0, 0, 0x5, 0, 0, 0, 0, 0, 0, 0], "type 0, aux 5 kept, flag 0");
    assert_eq!(MarkerInventory::from_table(&table).unwrap().count_entries(0x2, 0x5), 0);
}

#[test]
fn part_unlocks_match_on_aux_except_type_0xe_with_aux_0() {
    let mut inventory = MarkerInventory::empty();
    inventory.add_entries(0x2, 0x5, 1);
    inventory.add_entries(0xE, 0x3, 1);
    assert_eq!(inventory.count_entries(0x2, 0), 0, "type 2 needs aux 5");
    assert_eq!(inventory.count_entries(0x2, 0x5), 1);
    assert_eq!(inventory.count_entries(0xE, 0), 1, "type 0xE with aux 0 matches any aux");
    assert_eq!(inventory.count_entries(0xE, 0x4), 0, "type 0xE with another aux is exact");
    assert_eq!(inventory.count_entries(0xE, 0x3), 1);
}

#[test]
fn take_entry_with_the_wildcard_removes_the_first_active_match() {
    let mut inventory = MarkerInventory::empty();
    inventory.add_entries(0xE, 0x3, 1);
    inventory.add_entries(0xE, 0x4, 1);
    assert!(inventory.take_entry(0xE, 0));
    assert_eq!(inventory.count_entries(0xE, 0x3), 0, "the first match goes");
    assert_eq!(inventory.count_entries(0xE, 0x4), 1);
}

#[test]
fn the_kinds_have_the_documented_types() {
    assert_eq!(MarkerKind::JailFree.code(), 0x11);
    assert_eq!(MarkerKind::ExtraStrike.code(), 0x14);
    assert_eq!(MarkerKind::ImpoundRelease.code(), 0x15);
}
