use super::*;
use crate::career::CareerData;
use nfsmw_career::blacklist::BinDef;
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
