use super::*;
use crate::save::layout::profile;
use crate::savefile::autosave::AutosaveManager;
use crate::settings::{CareerSettings, Flags};

const HASH: u32 = 0xA1B2_C3D4;

/// A consistent file: a settings block with a valid gameplay blob, a stable hash and a fixed pattern elsewhere.
fn good_file() -> Vec<u8> {
    let mut buffer: Vec<u8> = (0..PROFILE_LEN).map(|i| (i % 199) as u8).collect();
    buffer[..16].fill(0);
    let mut settings = CareerSettings::new_career();
    settings.flags = Flags(0x1D_F7F3);
    settings.gameplay_blob[0x10..0x14].copy_from_slice(&BLOB_TAG.to_le_bytes());
    settings.gameplay_blob[0x14..0x18].copy_from_slice(&8u32.to_le_bytes());
    settings.refresh_gameplay_digest();
    settings.write_into_profile(&mut buffer).unwrap();
    buffer[profile::DEFAULT_STABLE_HASH..profile::DEFAULT_STABLE_HASH + 4].copy_from_slice(&HASH.to_le_bytes());
    ProfileSave::from_profile_buffer(&buffer).unwrap().to_bytes()
}

/// Recomputes the header checksum so a change to bytes 0..0x18 stays "valid".
fn fix_header_crc(file: &mut [u8]) {
    let crc = container_crc(&file[..header::HEADER_CRC]).unwrap();
    file[header::HEADER_CRC..header::HEADER_CRC + 4].copy_from_slice(&crc.to_le_bytes());
}

/// Rewrites the MD5 at the end of the profile.
fn fix_md5(file: &mut Vec<u8>) {
    let save = ProfileSave::from_unchecked(file.clone());
    *file = save.to_bytes();
}

#[test]
fn a_consistent_file_passes_and_reports_a_good_blob() {
    let report = check_load(&good_file(), HASH).unwrap();
    assert!(report.integrity.all_ok());
    assert_eq!(report.blob, BlobStatus::Accepted);
    assert_eq!(report.save.to_bytes(), good_file());
}

#[test]
fn the_error_codes_are_the_library_ones() {
    assert_eq!((CODE_SHORT_READ, CODE_BAD_CONTAINER, CODE_MISSING), (7, 8, 9));
    assert_eq!(LoadError::ShortRead.library_code(), Some(7));
    assert_eq!(LoadError::BadContainer.library_code(), Some(8));
    assert_eq!(LoadError::BadDescriptor.library_code(), None);
    assert!(LoadError::ShortRead.shows_unable_prompt());
    assert!(!LoadError::Md5Mismatch.shows_unable_prompt());
}

#[test]
fn a_file_shorter_than_the_header_is_a_short_read() {
    assert_eq!(check_load(&[], HASH).unwrap_err(), LoadError::ShortRead);
    assert_eq!(check_load(&good_file()[..0x1B], HASH).unwrap_err(), LoadError::ShortRead);
}

#[test]
fn a_truncated_file_fails_the_total_size_check_first() {
    let file = good_file();
    assert_eq!(check_load(&file[..file.len() - 1], HASH).unwrap_err(), LoadError::BadContainer);
    assert_eq!(check_load(&file[..0x1C], HASH).unwrap_err(), LoadError::BadContainer);
}

#[test]
fn the_magic_and_the_header_checksum_are_enforced() {
    let mut bad_magic = good_file();
    bad_magic[0] ^= 0xFF;
    fix_header_crc(&mut bad_magic);
    assert_eq!(check_load(&bad_magic, HASH).unwrap_err(), LoadError::BadContainer);
    let mut bad_crc = good_file();
    bad_crc[header::HEADER_CRC] ^= 1;
    assert_eq!(check_load(&bad_crc, HASH).unwrap_err(), LoadError::BadContainer);
}

#[test]
fn the_descriptor_and_profile_sizes_are_enforced() {
    let mut bad_desc = good_file();
    bad_desc[header::DESCRIPTOR_SIZE] = 9;
    fix_header_crc(&mut bad_desc);
    assert_eq!(check_load(&bad_desc, HASH).unwrap_err(), LoadError::BadContainer);
    let mut bad_profile = good_file();
    bad_profile[header::PROFILE_SIZE] ^= 1;
    fix_header_crc(&mut bad_profile);
    assert_eq!(check_load(&bad_profile, HASH).unwrap_err(), LoadError::BadContainer);
}

#[test]
fn a_file_longer_than_its_sizes_describe_is_refused() {
    let mut file = good_file();
    file.push(0);
    let total = file.len() as u32;
    file[header::TOTAL_SIZE..header::TOTAL_SIZE + 4].copy_from_slice(&total.to_le_bytes());
    fix_header_crc(&mut file);
    assert_eq!(check_load(&file, HASH).unwrap_err(), LoadError::BadContainer);
}

#[test]
fn a_wrong_descriptor_checksum_or_profile_checksum_is_ignored() {
    let mut file = good_file();
    file[header::DESCRIPTOR_CRC] ^= 1;
    file[header::PROFILE_CRC] ^= 1;
    fix_header_crc(&mut file);
    let report = check_load(&file, HASH).unwrap();
    assert!(!report.integrity.descriptor_crc && !report.integrity.profile_crc);
    assert!(report.integrity.profile_md5);
}

#[test]
fn an_editor_that_changes_the_profile_without_the_checksum_words_still_loads() {
    let mut file = good_file();
    let mut settings = ProfileSave::from_unchecked(file.clone()).settings().unwrap();
    settings.cash = 999_999;
    let save_with_cash = {
        let mut s = ProfileSave::from_unchecked(file.clone());
        s.set_settings(&settings).unwrap();
        s
    };
    file = save_with_cash.raw().to_vec();
    fix_md5(&mut file); // the MD5 is fixed; the profile checksum word is then also fixed by to_bytes
    let profile_crc_before = file[header::PROFILE_CRC..header::PROFILE_CRC + 4].to_vec();
    file[header::PROFILE_CRC..header::PROFILE_CRC + 4].copy_from_slice(&[0; 4]);
    fix_header_crc(&mut file);
    let report = check_load(&file, HASH).unwrap();
    assert!(!report.integrity.profile_crc);
    assert_eq!(report.save.settings().unwrap().cash, 999_999);
    assert_ne!(profile_crc_before, vec![0; 4]);
}

#[test]
fn the_descriptor_type_and_size_are_game_checks() {
    let mut wrong_type = good_file();
    wrong_type[header::TYPE_ID] ^= 1;
    assert_eq!(check_load(&wrong_type, HASH).unwrap_err(), LoadError::BadDescriptor);
    let mut wrong_size = good_file();
    wrong_size[header::DESCRIPTOR_PROFILE_SIZE] ^= 1;
    assert_eq!(check_load(&wrong_size, HASH).unwrap_err(), LoadError::BadDescriptor);
}

#[test]
fn a_different_default_stable_hash_stops_the_load() {
    assert_eq!(check_load(&good_file(), HASH ^ 1).unwrap_err(), LoadError::StableHashMismatch);
}

#[test]
fn the_md5_is_checked_last_so_the_stable_hash_error_comes_first() {
    let mut file = good_file();
    file[PROFILE_AT + 0x4010] ^= 1; // inside the md5 range
    assert_eq!(check_load(&file, HASH ^ 1).unwrap_err(), LoadError::StableHashMismatch);
    assert_eq!(check_load(&file, HASH).unwrap_err(), LoadError::Md5Mismatch);
}

#[test]
fn the_container_checks_come_before_the_game_checks() {
    let mut file = good_file();
    file[header::TYPE_ID] ^= 1;
    file[PROFILE_AT + 0x4010] ^= 1;
    file[0] ^= 1;
    assert_eq!(check_load(&file, HASH ^ 1).unwrap_err(), LoadError::BadContainer);
}

#[test]
fn a_bad_gameplay_blob_is_skipped_silently_and_the_file_still_loads() {
    let mut file = good_file();
    // Break the blob tag, then repair the profile MD5 so only the blob is bad.
    file[PROFILE_AT + 0x10 + 0x10] ^= 1;
    fix_md5(&mut file);
    let report = check_load(&file, HASH).unwrap();
    assert_eq!(report.blob, BlobStatus::Skipped);
}

#[test]
fn the_blob_needs_version_8_the_tag_and_its_own_md5() {
    let blob_with = |version: u32, tag: u32, refresh: bool| {
        let mut buffer = ProfileSave::from_unchecked(good_file()).profile_buffer().to_vec();
        let mut settings = CareerSettings::from_profile_bytes(&buffer).unwrap();
        settings.gameplay_blob[0x10..0x14].copy_from_slice(&tag.to_le_bytes());
        settings.gameplay_blob[0x14..0x18].copy_from_slice(&version.to_le_bytes());
        settings.gameplay_blob[0x100] ^= 0x55;
        if refresh {
            settings.refresh_gameplay_digest();
        }
        settings.write_into_profile(&mut buffer).unwrap();
        let file = ProfileSave::from_profile_buffer(&buffer).unwrap().to_bytes();
        check_load(&file, HASH).unwrap().blob
    };
    assert_eq!(blob_with(8, BLOB_TAG, true), BlobStatus::Accepted);
    assert_eq!(blob_with(9, BLOB_TAG, true), BlobStatus::Accepted, "8 or more");
    assert_eq!(blob_with(7, BLOB_TAG, true), BlobStatus::Skipped);
    assert_eq!(blob_with(8, BLOB_TAG ^ 1, true), BlobStatus::Skipped);
    assert_eq!(blob_with(8, BLOB_TAG, false), BlobStatus::Skipped, "stale blob digest");
}

#[test]
fn the_flags_word_is_merged_not_replaced() {
    let report = check_load(&good_file(), HASH).unwrap();
    let merged = report.merged_flags(Flags(0x2).with(Flags::GAME_OVER));
    assert_eq!(merged, Flags(0x1D_F7F3 | 0x2));
    assert!(!merged.contains(Flags::GAME_OVER));
}

#[test]
fn a_successful_load_arms_quiet_mode_only_with_the_option_on() {
    let report = check_load(&good_file(), HASH).unwrap();
    let mut profile = ProfileState::new();
    let mut on = AutosaveManager { off_by_error: true, ..AutosaveManager::default() };
    assert!(report.apply_success(&mut profile, &mut on));
    assert!(profile.loaded && !on.off_by_error && on.option_on);
    let mut off = AutosaveManager { option_on: false, off_by_error: true, ..AutosaveManager::default() };
    let mut profile2 = ProfileState::new();
    assert!(!report.apply_success(&mut profile2, &mut off));
    assert!(profile2.loaded && !off.off_by_error && !off.option_on);
}
