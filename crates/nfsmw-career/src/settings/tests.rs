use super::*;

/// A settings block with a recognisable value in every region.
fn synthetic_block() -> Vec<u8> {
    let mut block = vec![0u8; SETTINGS_BLOCK_LEN];
    for (i, b) in block.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    block
}

#[test]
fn new_career_has_the_documented_defaults() {
    let s = CareerSettings::new_career();
    assert_eq!(s.car_handle, 0x38A3_C42C);
    assert_eq!(s.stage, 16);
    assert_eq!(s.cash, 0);
    assert_eq!(s.flags, Flags::CAREER_STARTED);
    assert!(s.has_career());
    assert_eq!(s.sms.counter, 0);
    assert_eq!(s.sms.slots[149].id, 149);
    assert_eq!(PROLOGUE_START_NODE, "16.1.0");
}

#[test]
fn the_block_has_the_documented_size_and_round_trips_every_byte() {
    let block = synthetic_block();
    let s = CareerSettings::from_block_bytes(&block).unwrap();
    assert_eq!(s.to_block_bytes(), block);
}

#[test]
fn fields_are_read_from_the_packed_offsets() {
    // File offsets minus 0x34: handle 0x4034, stage 0x4038, cash 0x4039, unknown 0x403D, flags 0x403F,
    // SMS 0x4043, counter 0x429B, serial 0x429D.
    let mut block = vec![0u8; SETTINGS_BLOCK_LEN];
    block[0x4000..0x4004].copy_from_slice(&0x1234_5678u32.to_le_bytes());
    block[0x4004] = 15;
    block[0x4005..0x4009].copy_from_slice(&44_937_000u32.to_le_bytes());
    block[0x4009..0x400B].copy_from_slice(&30_372u16.to_le_bytes());
    block[0x400B..0x400F].copy_from_slice(&0x001D_F7F3u32.to_le_bytes());
    block[0x400F + 4 * 15..0x400F + 4 * 15 + 4].copy_from_slice(&[15, 2, 1, 0]);
    block[0x4267..0x4269].copy_from_slice(&74u16.to_le_bytes());
    block[0x4269..0x4269 + 8].copy_from_slice(b"MW-1-ABC");
    let s = CareerSettings::from_block_bytes(&block).unwrap();
    assert_eq!(s.car_handle, 0x1234_5678);
    assert_eq!(s.stage, 15);
    assert_eq!(s.cash, 44_937_000);
    assert_eq!(s.unknown_0x10, 30_372);
    assert_eq!(s.flags, Flags(0x001D_F7F3));
    assert_eq!(s.sms.slots[15], SmsSlot { id: 15, state: 2, sequence: 1 });
    assert_eq!(s.sms.counter, 74);
    assert_eq!(s.serial_text(), "MW-1-ABC");
}

#[test]
fn the_block_ends_exactly_where_the_next_profile_block_starts() {
    // 0x34 + 23,033 = 0x5A2D, the default stable hash.
    assert_eq!(0x34 + SETTINGS_BLOCK_LEN, 0x5A2D);
    assert_eq!(
        GAMEPLAY_BLOB_LEN
            + 4
            + 1
            + 4
            + 2
            + 4
            + SMS_TABLE_LEN
            + 2
            + 16
            + NODE_LIST_LEN
            + SHOP_UNLOCK_LEN
            + POPUP_QUEUE_LEN,
        SETTINGS_BLOCK_LEN
    );
}

#[test]
fn a_short_block_is_refused() {
    let err = CareerSettings::from_block_bytes(&[0; SETTINGS_BLOCK_LEN - 1]).unwrap_err();
    assert_eq!(err.need, SETTINGS_BLOCK_LEN);
    assert_eq!(err.got, SETTINGS_BLOCK_LEN - 1);
}

#[test]
fn the_profile_wrapper_reads_and_writes_at_offset_0x10() {
    let mut profile = vec![0xEEu8; 0xF848];
    let mut s = CareerSettings::new_career();
    s.cash = 1234;
    s.write_into_profile(&mut profile).unwrap();
    assert!(profile[..0x10].iter().all(|&b| b == 0xEE));
    assert!(profile[0x10 + SETTINGS_BLOCK_LEN..].iter().all(|&b| b == 0xEE));
    // cash is at file 0x4039 = profile buffer 0x4015.
    assert_eq!(&profile[0x4015..0x4019], &1234u32.to_le_bytes());
    assert_eq!(CareerSettings::from_profile_bytes(&profile).unwrap(), s);
}

#[test]
fn spending_clamps_at_zero() {
    let mut s = CareerSettings::new_career();
    s.cash = 500;
    assert!(s.can_afford(500));
    assert!(s.spend_cash(200));
    assert_eq!(s.cash, 300);
    assert!(!s.spend_cash(301));
    assert_eq!(s.cash, 0);
    assert!(s.spend_cash(0));
}

#[test]
fn adding_cash_saturates_instead_of_wrapping() {
    let mut s = CareerSettings::new_career();
    assert_eq!(s.add_cash(25_000), 25_000);
    s.cash = u32::MAX - 1;
    assert_eq!(s.add_cash(10), u32::MAX);
}

#[test]
fn the_serial_is_truncated_and_padded() {
    let mut s = CareerSettings::new_career();
    s.set_serial("MW-123456-ABCDXXX-and-more");
    assert_eq!(s.serial_text(), "MW-123456-ABCDXX");
    s.set_serial("MW-1");
    assert_eq!(&s.serial[4..], &[0; 12]);
}

#[test]
fn the_gameplay_digest_is_checked_and_refreshed() {
    let mut s = CareerSettings::new_career();
    assert!(!s.gameplay_digest_ok());
    s.refresh_gameplay_digest();
    assert!(s.gameplay_digest_ok());
    s.gameplay_blob[0x100] ^= 1;
    assert!(!s.gameplay_digest_ok());
}
