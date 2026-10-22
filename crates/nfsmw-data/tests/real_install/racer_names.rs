//! The fixed racer names and cars of the career races: `RacerName` of the prologue's characters, the Blacklist bosses'
//! preset rides and the leaderboard names of the language table (`docs/specs/racer-names.md`).

use blackbox_text::{StringTable, label_hash};
use nfsmw_data::car::CarTables;
use nfsmw_data::gameplay::{self, GameplayData, Race};
use nfsmw_data::shared;

use crate::install;

/// Bin, leaderboard name, preset ride and the car type the preset names, for the 15 Blacklist rivals.
const RIVALS: [(u8, &str, &str, &str); 15] = [
    (1, "RAZOR", "E3_DEMO_BMW", "BMWM3GTRE46"),
    (2, "BULL", "BL2", "SLR"),
    (3, "RONNIE", "BL3", "DB9"),
    (4, "JV", "BL4", "VIPER"),
    (5, "WEBSTER", "BL5", "CORVETTE"),
    (6, "MING", "BL6", "GALLARDO"),
    (7, "KAZE", "BL7", "CLK500"),
    (8, "JEWELS", "BL8", "MUSTANGGT"),
    (9, "EARL", "BL9", "LANCEREVO8"),
    (10, "BARON", "BL10", "CAYMANS"),
    (11, "BIG LOU", "BL11", "ECLIPSEGT"),
    (12, "IZZY", "BL12", "RX8"),
    (13, "VIC", "BL13", "SUPRA"),
    (14, "TAZ", "BL14", "IS300"),
    (15, "SONNY", "BL15", "GTI"),
];

/// Event, the opponent's collection, its fixed name (`None` draws one) and its preset ride, for the prologue races
/// that have opponents.
const PROLOGUE: [(&str, &str, Option<&str>, &str); 7] = [
    ("16_1_0_partial_dday", "razor", Some("RAZOR"), "RAZORMUSTANG"),
    ("16_2_2_sprint", "ronnie_mccrae", Some("RONNIE"), "DDAYSUPRA"),
    ("16_2_3_sprint", "toru_sato", Some("BULL"), "BL2"),
    ("16_2_1_d_day_intro", "opponent", Some("RAZOR"), "RAZORMUSTANG"),
    ("16_1_1_circuit", "opponent", Some("ROG"), "OPM_MUSTANG_BOSS"),
    ("16_1_1_circuit", "opponent2", None, "OPM_WRX_VERSION2"),
    ("16_1_1_circuit", "opponent3", None, "OPM_RX8_Version3"),
];

fn english() -> StringTable {
    let dir = install().expect("an install");
    StringTable::from_file(&shared::language(&dir).expect("English.bin")).expect("language table")
}

fn bin_of(race: &Race) -> Option<u8> {
    race.name.strip_prefix("race_bin_")?.split('/').next()?.parse().ok()
}

fn opponents<'a>(data: &'a GameplayData, race: &Race) -> Vec<&'a gameplay::Character> {
    race.params.opponents.iter().filter_map(|key| data.character(*key)).collect()
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_prologue_characters_carry_their_authored_names_and_cars() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let tables = CarTables::load(&dir).expect("car tables");
    let table = english();
    for (event, who, name, preset) in PROLOGUE {
        let race = data.race_named(event).unwrap_or_else(|| panic!("no race {event}"));
        assert!(!race.story.boss_race, "{event} is not a boss race: its racers go by RacerName");
        let character = opponents(&data, race)
            .into_iter()
            .find(|c| c.name.ends_with(&format!("/{who}")))
            .unwrap_or_else(|| panic!("{event} has no opponent {who}"));
        assert_eq!(character.preset_ride.as_deref(), Some(preset), "{event}/{who}");
        assert!(tables.preset(preset).is_some(), "{preset} is a preset ride of the install");
        let shown = character
            .racer_name
            .as_deref()
            .map(|label| table.get(label_hash(label)).unwrap_or_else(|| panic!("{event}/{who}: no text for {label}")));
        assert_eq!(shown.as_deref(), name, "{event}/{who}");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_other_prologue_racers_and_the_street_rivals_have_no_fixed_name() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let named: Vec<&str> = data.characters.iter().filter(|c| c.racer_name.is_some()).map(|c| c.name.as_str()).collect();
    assert_eq!(named.len(), 5, "{named:?}");
    for race in data.races.iter().filter(|r| r.story.boss_race) {
        for character in opponents(&data, race) {
            assert_eq!(character.racer_name, None, "{}: a boss race names its racer from the bin", race.name);
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_boss_race_has_its_rival_alone_with_the_rivals_name_and_car() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let tables = CarTables::load(&dir).expect("car tables");
    let table = english();
    let mut per_bin = [0usize; 16];
    for race in data.races.iter().filter(|r| r.story.boss_race && !r.params.opponents.is_empty()) {
        let bin = bin_of(race).unwrap_or_else(|| panic!("{} is not in a numbered bin", race.name));
        let (_, shown, preset, car) = RIVALS.iter().find(|r| r.0 == bin).unwrap_or_else(|| panic!("bin {bin}"));
        let rivals = opponents(&data, race);
        assert_eq!(rivals.len(), 1, "{}: the rival races alone", race.name);
        assert_eq!(rivals[0].preset_ride.as_deref(), Some(*preset), "{}", race.name);
        assert_eq!(tables.preset(preset).map(|p| p.car_type_name.as_str()), Some(*car), "{}", race.name);
        let label = format!("BLACKLIST_RIVAL_{bin:02}_LEADERBOARD");
        assert_eq!(table.get(label_hash(&label)).as_deref(), Some(*shown), "{label}");
        per_bin[usize::from(bin)] += 1;
    }
    assert!(per_bin[1..=15].iter().all(|n| *n >= 2), "every rival has boss races: {per_bin:?}");
    assert_eq!(per_bin.iter().sum::<usize>(), 37, "{per_bin:?}");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_language_table_has_a_leaderboard_name_for_each_rival_and_the_prologue() {
    let Some(_) = install() else { return };
    let table = english();
    for (bin, shown, _, _) in RIVALS {
        let label = format!("BLACKLIST_RIVAL_{bin:02}_LEADERBOARD");
        assert_eq!(table.get(label_hash(&label)).as_deref(), Some(shown), "{label}");
    }
    assert_eq!(table.get(label_hash("BLACKLIST_RIVAL_16_LEADERBOARD")).as_deref(), Some("ROG"));
}
