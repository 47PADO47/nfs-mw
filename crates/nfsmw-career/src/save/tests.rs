use super::layout::{FILE_LEN, PROFILE_AT, PROFILE_LEN, header, profile};
use super::*;
use crate::settings::{CareerSettings, Flags, SmsSlot};
use crate::stable::{CarRecord, CareerRecord, Stable};

/// A profile buffer with a recognisable pattern everywhere, a real settings block and a real stable.
fn synthetic_buffer() -> Vec<u8> {
    let mut buffer: Vec<u8> = (0..PROFILE_LEN).map(|i| (i * 7 % 241) as u8).collect();
    buffer[..16].fill(0);
    let mut settings = CareerSettings::new_career();
    settings.cash = 44_937_000;
    settings.stage = 15;
    settings.flags = Flags(0x001D_F7F3);
    settings.sms.slots[15] = SmsSlot { id: 15, state: 2, sequence: 1 };
    settings.sms.counter = 1;
    settings.write_into_profile(&mut buffer).unwrap();
    let mut stable = Stable::empty();
    stable.cars[3] = CarRecord { handle: settings.car_handle, flags: 0xF_0022, career_index: 1, ..CarRecord::free() };
    stable.careers[1] = CareerRecord { bounty: 4200, evaded: 7, ..CareerRecord::new(1) };
    stable.sold.bounty = 43_116_691;
    let at = profile::STABLE;
    buffer[at..at + crate::stable::STABLE_LEN].copy_from_slice(&stable.to_bytes());
    buffer[profile::NAME..profile::NAME + 8].copy_from_slice(b"mwo\0\0\0\0\0");
    buffer[profile::COMPLETED_ONCE] = 1;
    buffer
}

fn synthetic_save() -> ProfileSave {
    ProfileSave::from_profile_buffer(&synthetic_buffer()).unwrap()
}

#[test]
fn a_built_file_has_the_documented_header_and_consistent_checks() {
    let save = synthetic_save();
    let file = save.raw();
    assert_eq!(file.len(), 0xF86C);
    assert_eq!(&file[..4], b"20CM");
    assert_eq!(u32::from_le_bytes(file[4..8].try_into().unwrap()), 0xF86C);
    assert_eq!(u32::from_le_bytes(file[8..12].try_into().unwrap()), 8);
    assert_eq!(u32::from_le_bytes(file[12..16].try_into().unwrap()), 0xF848);
    assert_eq!(u32::from_le_bytes(file[0x10..0x14].try_into().unwrap()), 0x28A9_C0BB, "the descriptor checksum");
    assert_eq!(u32::from_le_bytes(file[0x1C..0x20].try_into().unwrap()), 0x10D);
    assert_eq!(u32::from_le_bytes(file[0x20..0x24].try_into().unwrap()), 0xF848);
    assert!(save.integrity().all_ok());
}

#[test]
fn a_consistent_file_parses_and_comes_back_byte_for_byte() {
    let built = synthetic_save();
    let parsed = ProfileSave::parse(built.raw()).unwrap();
    assert_eq!(parsed.to_bytes(), built.raw());
    assert_eq!(parsed.settings().unwrap().to_block_bytes(), built.settings().unwrap().to_block_bytes());
}

#[test]
fn reading_and_writing_the_blocks_changes_nothing_else() {
    let built = synthetic_save();
    let mut save = ProfileSave::parse(built.raw()).unwrap();
    let settings = save.settings().unwrap();
    let stable = save.stable().unwrap();
    save.set_settings(&settings).unwrap();
    save.set_stable(&stable);
    assert_eq!(save.to_bytes(), built.raw());
}

#[test]
fn the_parsed_blocks_have_the_values_that_were_written() {
    let save = synthetic_save();
    let settings = save.settings().unwrap();
    assert_eq!((settings.cash, settings.stage, settings.flags), (44_937_000, 15, Flags(0x001D_F7F3)));
    assert_eq!(settings.sms.slots[15], SmsSlot { id: 15, state: 2, sequence: 1 });
    let stable = save.stable().unwrap();
    assert_eq!(stable.sold.bounty, 43_116_691);
    assert_eq!(stable.career_for_handle(settings.car_handle).map(|r| r.bounty), Some(4200));
    assert_eq!(stable.total_bounty(), 43_116_691 + 4200);
    assert_eq!(save.alias(), "mwo");
    assert!(save.career_completed_once());
}

#[test]
fn the_editor_offsets_land_on_the_fields_they_name() {
    let save = synthetic_save();
    let file = save.raw();
    // Money at file 0x4039 is settings +0x0C; the sold-history bounty at 0xE865; the first career record at 0xE2ED.
    assert_eq!(u32::from_le_bytes(file[0x4039..0x403D].try_into().unwrap()), 44_937_000);
    assert_eq!(u32::from_le_bytes(file[0xE865..0xE869].try_into().unwrap()), 43_116_691);
    assert_eq!(file[0xE2ED + 0x38], 1, "career record 1 starts with its own index");
    assert_eq!(u32::from_le_bytes(file[0xE2ED + 0x38 + 0x10..0xE2ED + 0x38 + 0x14].try_into().unwrap()), 4200);
}

#[test]
fn editing_cash_rewrites_the_checksums_and_leaves_every_other_byte() {
    let built = synthetic_save();
    let mut save = ProfileSave::parse(built.raw()).unwrap();
    let mut settings = save.settings().unwrap();
    settings.cash = 12_345;
    save.set_settings(&settings).unwrap();
    assert!(!save.integrity().profile_md5, "the MD5 is stale until the file is written");
    let out = save.to_bytes();
    let reparsed = ProfileSave::parse(&out).unwrap();
    assert!(reparsed.integrity().all_ok());
    assert_eq!(reparsed.settings().unwrap().cash, 12_345);
    let changed: Vec<usize> = (0..FILE_LEN).filter(|&i| out[i] != built.raw()[i]).collect();
    let allowed = |i: usize| {
        (0x4039..0x403D).contains(&i)
            || (header::DESCRIPTOR_CRC..header::HEADER_CRC + 4).contains(&i)
            || (PROFILE_AT + profile::MD5..FILE_LEN).contains(&i)
    };
    assert!(changed.iter().all(|&i| allowed(i)), "unexpected changes at {changed:?}");
    assert!(changed.contains(&0x4039));
}

#[test]
fn unknown_bytes_inside_the_blocks_survive_an_edit() {
    let built = synthetic_save();
    let mut save = ProfileSave::parse(built.raw()).unwrap();
    let mut stable = save.stable().unwrap();
    stable.careers[1].unknown_08 = [9, 8, 7, 6];
    stable.cars[3].tail = [0xAA, 0xBB];
    save.set_stable(&stable);
    let mut settings = save.settings().unwrap();
    settings.unknown_0x10 = 30_372;
    save.set_settings(&settings).unwrap();
    let out = ProfileSave::parse(&save.to_bytes()).unwrap();
    assert_eq!(out.stable().unwrap().careers[1].unknown_08, [9, 8, 7, 6]);
    assert_eq!(out.stable().unwrap().cars[3].tail, [0xAA, 0xBB]);
    assert_eq!(out.settings().unwrap().unknown_0x10, 30_372);
    assert_eq!(out.default_stable_hash(), built.default_stable_hash());
    assert_eq!(&out.profile_buffer()[..0x10], &[0; 16]);
}

#[test]
fn a_file_with_a_wrong_length_magic_or_header_is_refused() {
    let good = synthetic_save().to_bytes();
    assert_eq!(ProfileSave::parse(&good[..good.len() - 1]).unwrap_err(), SaveError::BadLength(FILE_LEN - 1));
    let mut bad_magic = good.clone();
    bad_magic[0] = b'X';
    assert!(matches!(ProfileSave::parse(&bad_magic), Err(SaveError::BadMagic(_))));
    let mut bad_size = good.clone();
    bad_size[header::PROFILE_SIZE] ^= 1;
    assert_eq!(ProfileSave::parse(&bad_size).unwrap_err(), SaveError::BadHeader("profile size"));
    let mut bad_type = good.clone();
    bad_type[header::TYPE_ID] ^= 1;
    assert_eq!(ProfileSave::parse(&bad_type).unwrap_err(), SaveError::BadHeader("profile type id"));
}

#[test]
fn the_header_checksum_and_the_md5_are_enforced() {
    let good = synthetic_save().to_bytes();
    let mut bad_header_crc = good.clone();
    bad_header_crc[header::HEADER_CRC] ^= 1;
    assert_eq!(ProfileSave::parse(&bad_header_crc).unwrap_err(), SaveError::HeaderChecksum);
    let mut bad_md5 = good.clone();
    bad_md5[PROFILE_AT + 0x4000] ^= 1;
    assert_eq!(ProfileSave::parse(&bad_md5).unwrap_err(), SaveError::Md5Mismatch);
}

#[test]
fn the_version_field_is_outside_the_md5_range() {
    let mut file = synthetic_save().to_bytes();
    file[PROFILE_AT] = 7;
    let save = ProfileSave::parse(&file).unwrap();
    assert!(save.integrity().profile_md5);
    assert!(!save.integrity().profile_crc, "the profile checksum does cover it");
}

#[test]
fn a_wrong_profile_or_descriptor_checksum_is_accepted_and_repaired_on_write() {
    let good = synthetic_save().to_bytes();
    let mut file = good.clone();
    file[header::PROFILE_CRC] ^= 1;
    file[header::DESCRIPTOR_CRC] ^= 1;
    // The header checksum covers bytes 0..0x18, which include these two words: make it agree with them.
    let header_crc = crc::container_crc(&file[..header::HEADER_CRC]).unwrap();
    file[header::HEADER_CRC..header::HEADER_CRC + 4].copy_from_slice(&header_crc.to_le_bytes());
    let save = ProfileSave::parse(&file).unwrap();
    let integrity = save.integrity();
    assert!(!integrity.profile_crc && !integrity.descriptor_crc && integrity.header_crc);
    assert_eq!(save.to_bytes(), good);
}

#[test]
fn a_blank_profile_is_a_consistent_new_career() {
    let save = ProfileSave::new_blank("mwo").unwrap();
    assert_eq!(save.raw().len(), 0xF86C);
    assert!(save.integrity().all_ok());
    assert_eq!(save.alias(), "mwo");
    let settings = save.settings().unwrap();
    assert_eq!((settings.stage, settings.cash, settings.flags), (16, 0, Flags::CAREER_STARTED));
    assert!(settings.has_career());
    assert_eq!(save.stable().unwrap().total_bounty(), 0);
    assert!(!save.career_completed_once());
    assert_eq!(ProfileSave::parse(save.raw()).unwrap(), save);
}

#[test]
fn profile_completion_persists_independently_of_the_current_career_flags() {
    let mut save = ProfileSave::new_blank("mwo").unwrap();
    save.mark_career_completed();
    let bytes = save.to_bytes();
    let save = ProfileSave::parse(&bytes).unwrap();
    assert!(save.career_completed_once());
    assert!(!save.settings().unwrap().flags.contains(Flags::CAREER_COMPLETE));
    assert!(save.integrity().all_ok());
}

#[test]
fn a_blank_profile_keeps_only_the_first_32_name_bytes() {
    let long = "x".repeat(40);
    let save = ProfileSave::new_blank(&long).unwrap();
    assert_eq!(save.alias(), "x".repeat(32));
}

#[test]
fn a_profile_buffer_of_the_wrong_size_is_refused() {
    assert!(ProfileSave::from_profile_buffer(&[0; 10]).is_err());
}

#[test]
fn the_history_block_is_read_and_written_in_place() {
    use crate::rapsheet::{PursuitStats, case_id_text};
    let built = synthetic_save();
    let mut save = ProfileSave::parse(built.raw()).unwrap();
    let mut history = crate::rapsheet::PursuitHistory::default();
    let stats = PursuitStats { car_key: 7, bounty: 900, ticks: 8000, cops_involved: 4, ..PursuitStats::default() };
    history.record(&stats, false, case_id_text(b"1A2B3C4D"));
    save.set_history(&history).unwrap();
    let out = ProfileSave::parse(&save.to_bytes()).unwrap();
    let again = out.history().unwrap();
    assert_eq!(again.sums[0], history.sums[0]);
    assert_eq!(again.top[0].bounty, 900);
    // file offsets: the sums are at 0xE88E + 0xB44, the top 5 at 0xE88E + 0xA2C, the last car key at 0xE88E + 0xBD4.
    let file = out.raw();
    assert_eq!(&file[0xE88E + 0xA2C..0xE88E + 0xA2C + 3], b"MW-");
    assert_eq!(u32::from_le_bytes(file[0xE88E + 0xBD4..0xE88E + 0xBD8].try_into().unwrap()), 7);
}
