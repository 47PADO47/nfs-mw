use super::*;
use crate::career::CareerData;
use nfsmw_career::blacklist::BinDef;
use nfsmw_career::economy::PursuitResult;
use nfsmw_career::economy::markers::MarkerKind;
use nfsmw_career::milestones::speedtrap::SpeedTrapDef;
use nfsmw_career::milestones::{MilestoneDef, MilestoneState, StatType};
use nfsmw_career::settings::NewCareer;
use std::sync::Arc;

/// A bin with no races and no rival, only its number. The Blacklist needs bins 1 to 15 once each.
fn bin(number: u8) -> BinDef {
    BinDef {
        number,
        races: Vec::new(),
        races_needed: 0,
        challenges_needed: 0,
        bounty_needed: 0,
        rival_events: Vec::new(),
    }
}

/// The smallest career table: the 15 bins, no milestones, no traps, no cars.
fn data() -> Arc<CareerData> {
    Arc::new(CareerData {
        bins: (1..=15).map(bin).collect(),
        milestones: Vec::new(),
        traps: Vec::new(),
        cars: Vec::new(),
        heat: Default::default(),
        ranks: Default::default(),
        gameplay: Default::default(),
    })
}

/// The career table with one milestone and one speed trap, both in bin 1.
fn data_with_progress() -> Arc<CareerData> {
    Arc::new(CareerData {
        bins: (1..=15).map(bin).collect(),
        milestones: vec![MilestoneDef {
            key: 0x1234,
            bin: 1,
            stat: StatType::PursuitLength,
            threshold: 10.0,
            bounty: 500,
        }],
        traps: vec![SpeedTrapDef { key: 0x5678, bin: 1, index: 1, threshold_kmh: 120, bounty: 300 }],
        cars: Vec::new(),
        heat: Default::default(),
        ranks: Default::default(),
        gameplay: Default::default(),
    })
}

#[test]
fn a_new_careers_progress_survives_a_save_and_load() {
    let mut career = Career::new("mwo", data_with_progress(), &NewCareer::default()).unwrap();
    career.milestones.enter_bin(1);
    career.milestones.update_stat(StatType::PursuitLength, 20.0, true);
    career.milestones.on_pursuit_end(&PursuitResult::default(), true, 0.0, 0);
    assert_eq!(career.milestones.state_of(0x1234), Some(MilestoneState::Complete));
    let bytes = career.profile_bytes();
    let back = Career::from_profile_bytes("mwo", &bytes, data_with_progress()).unwrap();
    assert_eq!(back.milestones.state_of(0x1234), Some(MilestoneState::Complete));
    assert!(back.milestones.milestones()[0].value.is_some());
}

#[test]
fn a_new_career_has_its_milestone_block_laid_out() {
    let mut career = Career::new("mwo", data_with_progress(), &NewCareer::default()).unwrap();
    let bytes = career.profile_bytes();
    let back = Career::from_profile_bytes("mwo", &bytes, data_with_progress()).unwrap();
    assert_eq!(back.milestones.state_of(0x1234), Some(MilestoneState::Locked));
    assert!(!back.traps[0].done());
}

#[test]
fn a_new_career_starts_at_stage_16_with_no_cash() {
    let career = Career::new("mwo", data(), &NewCareer::default()).unwrap();
    assert_eq!(career.stage(), 16);
    assert_eq!(career.cash(), 0);
    assert_eq!(career.alias, "mwo");
}

#[test]
fn a_profile_written_and_read_back_keeps_cash_stage_and_alias() {
    let mut career = Career::new("mwo", data(), &NewCareer::default()).unwrap();
    career.settings.add_cash(12_345);
    let bytes = career.profile_bytes();
    assert_eq!(bytes.len(), 0xF86C);
    let back = Career::from_profile_bytes("mwo", &bytes, data()).unwrap();
    assert_eq!(back.cash(), 12_345);
    assert_eq!(back.stage(), 16);
    assert_eq!(back.alias, "mwo");
}

#[test]
fn writing_a_loaded_career_again_gives_the_same_file() {
    let mut career = Career::new("mwo", data(), &NewCareer::default()).unwrap();
    let bytes = career.profile_bytes();
    let mut back = Career::from_profile_bytes("mwo", &bytes, data()).unwrap();
    assert_eq!(back.profile_bytes(), bytes);
}

#[test]
fn a_bad_profile_file_is_refused() {
    let mut bytes = Career::new("mwo", data(), &NewCareer::default()).unwrap().profile_bytes();
    bytes[0] = b'X';
    assert!(Career::from_profile_bytes("mwo", &bytes, data()).is_err());
}

#[test]
fn a_new_career_holds_no_markers() {
    let career = Career::new("mwo", data(), &NewCareer::default()).unwrap();
    assert_eq!(career.markers.count(MarkerKind::JailFree), 0);
}

#[test]
fn held_markers_are_saved_with_the_profile_and_read_back() {
    let mut career = Career::new("mwo", data(), &NewCareer::default()).unwrap();
    career.markers.add(MarkerKind::JailFree, 4);
    career.markers.add(MarkerKind::ImpoundRelease, 1);
    let bytes = career.profile_bytes();
    let back = Career::from_profile_bytes("mwo", &bytes, data()).unwrap();
    assert_eq!(back.markers.count(MarkerKind::JailFree), 4);
    assert_eq!(back.markers.count(MarkerKind::ImpoundRelease), 1);
    assert_eq!(back.markers, career.markers);
}

/// Bin 14 with one free race and one boss race, so a career can win them.
fn data_with_races() -> Arc<CareerData> {
    let mut bins: Vec<BinDef> = (1..=15).map(bin).collect();
    bins[13] = BinDef {
        number: 14,
        races: vec!["14.1.1".into()],
        races_needed: 1,
        challenges_needed: 0,
        bounty_needed: 0,
        rival_events: vec!["14.8.1".into()],
    };
    let mut data = Arc::unwrap_or_clone(data());
    data.bins = bins;
    Arc::new(data)
}

#[test]
fn the_races_won_survive_a_save_and_load() {
    let mut career = Career::new("mwo", data_with_races(), &NewCareer::default()).unwrap();
    career.blacklist.enter_bin(14);
    let mut flags = career.settings.flags;
    career.blacklist.race_finished("14.1.1", true, &mut flags);
    assert_eq!(career.blacklist.bin(14).unwrap().races_won, ["14.1.1"]);
    let bytes = career.profile_bytes();
    let back = Career::from_profile_bytes("mwo", &bytes, data_with_races()).unwrap();
    assert_eq!(back.stage(), 14);
    let record = back.blacklist.bin(14).unwrap();
    assert_eq!(record.races_won, ["14.1.1"]);
    assert_eq!(
        record.state,
        Some(nfsmw_career::blacklist::BinState::RivalUnlocked),
        "the gate of the bin is met again"
    );
}

#[test]
fn a_rival_beaten_stays_beaten_after_a_save_and_load() {
    let mut career = Career::new("mwo", data_with_races(), &NewCareer::default()).unwrap();
    career.blacklist.enter_bin(14);
    let mut flags = career.settings.flags;
    career.blacklist.race_finished("14.1.1", true, &mut flags);
    career.blacklist.race_finished("14.8.1", true, &mut flags);
    assert_eq!(career.stage(), 13, "the walk went down to the next bin");
    let bytes = career.profile_bytes();
    let back = Career::from_profile_bytes("mwo", &bytes, data_with_races()).unwrap();
    assert_eq!(back.stage(), 13);
    let record = back.blacklist.bin(14).unwrap();
    assert_eq!(record.state, Some(nfsmw_career::blacklist::BinState::Done));
    assert_eq!(record.rival_races_won, ["14.8.1"]);
}

#[test]
fn a_career_with_no_races_won_writes_no_race_records() {
    let mut career = Career::new("mwo", data_with_races(), &NewCareer::default()).unwrap();
    let before = career.settings.nodes().len();
    career.profile_bytes();
    assert_eq!(career.settings.nodes().len(), before);
}

// Native options layout documented in docs/formats/career-save.md.
const AUTOSAVE_BYTE: usize = 0x5B31;

#[test]
fn batch18_loaded_autosave_option_controls_the_runtime_manager() {
    let original = Career::new("AUTO", data(), &NewCareer::default()).unwrap();
    let mut buffer = original.profile.profile_buffer().to_vec();
    for enabled in [false, true] {
        buffer[AUTOSAVE_BYTE] = u8::from(enabled);
        let bytes = ProfileSave::from_profile_buffer(&buffer).unwrap().to_bytes();
        let career = Career::from_profile_bytes("AUTO", &bytes, data()).unwrap();
        assert_eq!(career.autosave.option_on, enabled, "imported native option");
    }
}

#[test]
fn batch18_serialized_autosave_option_preserves_other_option_bytes() {
    let original = Career::new("AUTO", data(), &NewCareer::default()).unwrap();
    let mut buffer = original.profile.profile_buffer().to_vec();
    buffer[0x5B1D..0x5BA1].fill(0x9A);
    buffer[AUTOSAVE_BYTE] = 1;
    let bytes = ProfileSave::from_profile_buffer(&buffer).unwrap().to_bytes();
    let mut career = Career::from_profile_bytes("AUTO", &bytes, data()).unwrap();
    career.autosave.option_on = false;
    let saved = ProfileSave::parse(&career.profile_bytes()).unwrap();
    assert_eq!(saved.profile_buffer()[AUTOSAVE_BYTE], 0);
    for at in 0x5B1D..0x5BA1 {
        if at != AUTOSAVE_BYTE {
            assert_eq!(saved.profile_buffer()[at], 0x9A, "unknown option at {at:#x}");
        }
    }
    assert!(saved.integrity().all_ok());
}

#[test]
fn batch18_new_profile_autosave_matches_its_runtime_default() {
    let career = Career::new("AUTO", data(), &NewCareer::default()).unwrap();
    assert!(career.autosave.option_on);
    assert_eq!(career.profile.profile_buffer()[AUTOSAVE_BYTE], 1);
}

fn autosave_test_folder() -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("nfsmw-autosave-option-{}-{nonce}", std::process::id()))
}

#[test]
fn batch18_autosave_preference_is_durable_without_saving_unsaved_progress() {
    let root = autosave_test_folder();
    let profiles = crate::career::ProfileDir::new(&root);
    let mut career = Career::new("AUTO", data(), &NewCareer::default()).unwrap();
    let checkpoint_cash = career.settings.cash;
    profiles.write("AUTO", &career.profile_bytes()).unwrap();
    let mut other = Career::new("OTHER", data(), &NewCareer::default()).unwrap();
    profiles.write("OTHER", &other.profile_bytes()).unwrap();
    let other_bytes = profiles.read("OTHER").unwrap();
    career.settings.cash += 123;
    career.autosave.pending = true;
    career.autosave.armed = true;
    career.autosave.off_by_error = true;
    career.set_autosave_enabled(&profiles, false).unwrap();
    assert!(!career.autosave.option_on);
    assert!(!career.autosave.pending && !career.autosave.armed && !career.autosave.off_by_error);
    let loaded = Career::from_profile_bytes("AUTO", &profiles.read("AUTO").unwrap(), data()).unwrap();
    assert!(!loaded.autosave.option_on);
    assert_eq!(loaded.settings.cash, checkpoint_cash, "preference is not a progress checkpoint");
    assert_eq!(career.settings.cash, checkpoint_cash + 123, "live progress stays intact");
    assert_eq!(profiles.read("OTHER").unwrap(), other_bytes, "other alias stays untouched");
    career.set_autosave_enabled(&profiles, true).unwrap();
    let loaded = Career::from_profile_bytes("AUTO", &profiles.read("AUTO").unwrap(), data()).unwrap();
    assert!(loaded.autosave.option_on);
    assert_eq!(loaded.settings.cash, checkpoint_cash);
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn batch18_failed_autosave_preference_write_keeps_the_previous_choice() {
    let root = autosave_test_folder();
    std::fs::write(&root, b"blocked profile root").unwrap();
    let profiles = crate::career::ProfileDir::new(&root);
    let mut career = Career::new("AUTO", data(), &NewCareer::default()).unwrap();
    let before = career.autosave;
    let native = career.profile.to_bytes();
    assert!(career.set_autosave_enabled(&profiles, false).is_err());
    assert_eq!(career.autosave, before);
    assert_eq!(career.profile.to_bytes(), native);
    std::fs::remove_file(root).unwrap();
}
