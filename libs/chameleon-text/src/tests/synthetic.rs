use crate::{Error, StringEntry, StringTable};

fn entry(id: u32, text: &str) -> StringEntry {
    StringEntry { id, text: text.to_owned() }
}

fn sample() -> StringTable {
    StringTable::new(3, vec![entry(9, "neuf"), entry(2, "deux"), entry(2, "bis"), entry(5, "日本語 \u{1F698}")])
}

#[test]
fn sorts_by_id_and_keeps_duplicates_in_order() {
    let table = sample();
    let ids: Vec<u32> = table.entries().iter().map(|entry| entry.id).collect();
    assert_eq!(ids, [2, 2, 5, 9]);
    assert_eq!(table.get(2), Some("deux"));
    assert_eq!(table.all(2).len(), 2);
    assert_eq!(table.get(5), Some("日本語 \u{1F698}"));
    assert_eq!(table.get(4), None);
    assert!(table.all(100).is_empty());
}

#[test]
fn writes_what_it_reads() {
    let bytes = sample().to_bytes();
    assert_eq!(bytes.len() % 16, 0);
    assert_eq!(&bytes[..16], [3, 0, 0, 0, 4, 0, 0, 0, 16, 0, 0, 0, 32, 0, 0, 0]);
    let again = StringTable::parse(&bytes).unwrap();
    assert_eq!(again, sample());
    assert_eq!(again.to_bytes(), bytes);
}

#[test]
fn an_empty_table_is_a_header() {
    let table = StringTable::new(1, Vec::new());
    assert_eq!(table.to_bytes().len(), 16);
    assert_eq!(StringTable::parse(&table.to_bytes()).unwrap(), table);
}

#[test]
fn set_replaces_or_inserts_in_order() {
    let mut table = sample();
    assert_eq!(table.set(5, "cinq").as_deref(), Some("日本語 \u{1F698}"));
    assert_eq!(table.set(7, "sept"), None);
    let ids: Vec<u32> = table.entries().iter().map(|entry| entry.id).collect();
    assert_eq!(ids, [2, 2, 5, 7, 9]);
    assert_eq!(StringTable::parse(&table.to_bytes()).unwrap().get(7), Some("sept"));
}

#[test]
fn refuses_damaged_blocks() {
    assert!(matches!(StringTable::parse(&[0; 8]), Err(Error::Truncated { what: "header", .. })));
    let good = sample().to_bytes();
    let mut huge_count = good.clone();
    huge_count[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(StringTable::parse(&huge_count), Err(Error::Truncated { .. })));
    // The first record is at 16 + 4 * 4 = 32 (text offset, then length): point the text outside the block.
    let mut outside = good.clone();
    outside[32..36].copy_from_slice(&(good.len() as u32).to_le_bytes());
    assert!(matches!(StringTable::parse(&outside), Err(Error::Truncated { what: "text", .. })));
    // A text whose last unit is not NUL.
    let mut unterminated = good.clone();
    let text_at = u32::from_le_bytes([good[32], good[33], good[34], good[35]]) as usize;
    let units = u32::from_le_bytes([good[36], good[37], good[38], good[39]]) as usize;
    unterminated[text_at + 2 * (units - 1)] = b'x';
    assert!(matches!(StringTable::parse(&unterminated), Err(Error::Unterminated { index: 0 })));
    // A lone surrogate.
    let mut surrogate = good;
    surrogate[text_at..text_at + 2].copy_from_slice(&0xD800u16.to_le_bytes());
    assert!(matches!(StringTable::parse(&surrogate), Err(Error::BadText { index: 0 })));
}
