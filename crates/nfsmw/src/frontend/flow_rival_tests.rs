use nfsmw_career::blacklist::{BinDef, Blacklist};

use super::*;

/// The fifteen rivals, each with two boss races (`<bin>.8.1` first), the bin `without` with none.
fn blacklist(without: Option<u8>) -> Blacklist {
    let defs = (1..=15u8)
        .map(|number| BinDef {
            number,
            races: vec![format!("{number}.1.1")],
            races_needed: 1,
            challenges_needed: 1,
            bounty_needed: 1000 * u32::from(number),
            rival_events: if without == Some(number) {
                Vec::new()
            } else {
                vec![format!("{number}.8.1"), format!("{number}.8.2")]
            },
        })
        .collect();
    Blacklist::new(defs, 16).expect("fifteen bins")
}

#[test]
fn a_challenge_starts_the_first_boss_race_of_its_bin() {
    let bl = blacklist(None);
    assert_eq!(first_boss_race(&bl, 15).as_deref(), Some("15.8.1"));
    assert_eq!(first_boss_race(&bl, 3).as_deref(), Some("3.8.1"));
}

#[test]
fn a_bin_without_boss_races_or_outside_the_rivals_starts_nothing() {
    let bl = blacklist(Some(7));
    assert_eq!(first_boss_race(&bl, 7), None);
    assert_eq!(first_boss_race(&bl, 16), None, "the prologue is not a rival");
}
