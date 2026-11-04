use super::*;
use std::collections::HashSet;

/// The scene choice of a table scene.
fn bit(scene: &str) -> u8 {
    SCENE_BITS[nis_row(scene).unwrap_or_else(|| panic!("{scene} is not in the table"))]
}

#[test]
fn the_table_has_the_spec_rows_and_no_two_scenes_share_a_hash() {
    assert_eq!(NIS_SCENES.len(), 68);
    assert_eq!(NIS_SCENES[0], "GenericStart");
    assert_eq!(NIS_SCENES[17], "OPMRivalIntro");
    assert_eq!(NIS_SCENES[18], "IntroNisBL01");
    assert_eq!(NIS_SCENES[35], "IntroNisBL16");
    assert_eq!(NIS_SCENES[36], "ArrestF02");
    assert_eq!(NIS_SCENES[67], "EndingNis04");
    let hashes: HashSet<u32> = NIS_SCENES.iter().map(|name| bstring_hash(name)).collect();
    assert_eq!(hashes.len(), 68);
}

#[test]
fn a_scene_takes_the_branch_of_its_row() {
    assert_eq!(nis_branch("GenericStart"), Some(NisBranch::Start));
    assert_eq!(nis_branch("IntroNis05"), Some(NisBranch::Start));
    assert_eq!(nis_branch("Flythrough"), Some(NisBranch::Start));
    assert_eq!(nis_branch("IntroNisBL02"), Some(NisBranch::Blacklist));
    assert_eq!(nis_branch("IntroNisBL06End"), Some(NisBranch::Blacklist));
    assert_eq!(nis_branch("ArrestF02"), Some(NisBranch::End));
    assert_eq!(nis_branch("ArrestM23b"), Some(NisBranch::End));
    assert_eq!(nis_branch("EndingNis04"), Some(NisBranch::End));
}

#[test]
fn a_scene_outside_the_table_plays_no_stream() {
    assert_eq!(nis_row("FMVTest"), None);
    assert_eq!(nis_row("ReplayBurnout"), None);
    assert_eq!(nis_branch("WMBoat"), None);
}

#[test]
fn the_arrests_take_their_nis_select_end_choices() {
    let arrests = [
        ("ArrestM01", 0),
        ("ArrestM04", 1),
        ("ArrestM06", 2),
        ("ArrestM14", 3),
        ("ArrestM16", 4),
        ("ArrestM19", 5),
        ("ArrestM23", 6),
        ("ArrestF02", 7),
        ("ArrestF06", 8),
        ("ArrestF07", 9),
        ("ArrestF14", 10),
        ("ArrestF23", 11),
        ("ArrestF18", 14),
        ("ArrestM07", 15),
    ];
    for (scene, choice) in arrests {
        assert_eq!(bit(scene), choice, "{scene}");
        assert_eq!(bit(&format!("{scene}b")), choice, "{scene}b plays the take of its base");
    }
    let endings: Vec<u8> = (1..=4).map(|n| bit(&format!("EndingNis0{n}"))).collect();
    assert_eq!(endings, [17, 18, 19, 20]);
    let distinct: HashSet<u8> = SCENE_BITS[36..].iter().copied().collect();
    assert_eq!(distinct.len(), 18, "14 arrests and 4 endings");
    assert!(SCENE_BITS[36..].iter().all(|&b| b < 21), "NIS_Scene_End has 21 choices");
}

#[test]
fn the_start_and_blacklist_scenes_take_their_choices() {
    assert_eq!(bit("GenericStart"), 18);
    assert_eq!(bit("IntroNis01"), 0);
    assert_eq!(bit("IntroNis10"), 9);
    assert_eq!(bit("IntroNisDD"), 14);
    assert_eq!(bit("IntroNisDDEnd"), 19);
    assert_eq!(bit("IntroNisToll01"), 17);
    assert_eq!(bit("GarageEnter"), bit("GarageExit"));
    assert!(SCENE_BITS[..18].iter().all(|&b| b < 20), "NIS_Scene_Start has 20 choices");
    assert_eq!(bit("IntroNisBL01"), 0);
    assert_eq!(bit("IntroNisBL16"), 15);
    assert_eq!(bit("IntroNisBL06End"), bit("IntroNisBL06"));
    assert_eq!(bit("IntroNisBL07End"), bit("IntroNisBL07"));
    assert!(SCENE_BITS[18..36].iter().all(|&b| b < 19), "NIS_Blacklist has 19 choices");
}

#[test]
fn each_branch_asks_its_record_for_the_scene_track_0_and_the_complete_section() {
    let end = NisBranch::End.params(7);
    assert_eq!(NisBranch::End.event(), 0x4011);
    assert_eq!((end.get(1), end.get(2), end.get(3)), (Some(1 << 7), Some(1 << 13), Some(1)));
    let start = NisBranch::Start.params(4);
    assert_eq!(NisBranch::Start.event(), 0x4000);
    assert_eq!((start.get(1), start.get(2), start.get(3)), (Some(1 << 4), Some(1 << 13), Some(1)));
    let blacklist = NisBranch::Blacklist.params(5);
    assert_eq!(NisBranch::Blacklist.event(), 0x4014);
    assert_eq!((blacklist.get(1), blacklist.get(2), blacklist.get(3)), (Some(1 << 13), Some(1), Some(1 << 5)));
}

/// An event database of one-entry records: (event, phrase, tests as mask and argument number). Each record has one
/// block of weight 100 whose one entry reads its phrase for any voice. Layout: `docs/formats/speech-evt.md`.
fn events(records: &[(u16, u16, &[(u32, u8)])]) -> Vec<u8> {
    let mut out = vec![0u8; (0x18 + 2 * records.len()).next_multiple_of(4)];
    out[0x10..0x14].copy_from_slice(&(records.len() as u32).to_le_bytes());
    for (i, &(event, phrase, tests)) in records.iter().enumerate() {
        let word = u16::try_from(out.len() / 4).unwrap();
        out[0x18 + 2 * i..0x1A + 2 * i].copy_from_slice(&word.to_le_bytes());
        let mut record = vec![0u8; 16];
        record[..2].copy_from_slice(&event.to_le_bytes());
        record[6] = 1;
        record[12] = 4;
        record.extend_from_slice(&[0x39, 100, 1 << 2, 0, 0, 0, 0, 0, 3, 0, 0, 0]);
        record.extend_from_slice(&phrase.to_le_bytes());
        record.extend_from_slice(&[0, 0]);
        record.extend_from_slice(&(tests.len() as u32).to_le_bytes());
        for &(mask, argument) in tests {
            record.extend_from_slice(&mask.to_le_bytes());
            record.extend_from_slice(&[argument, 0xFF, 0xFF, 0]);
        }
        out.extend_from_slice(&record);
    }
    out
}

/// A speech index of banks for any voice: (phrase, the three selectors of each take). Layout: `docs/formats/audio.md`.
fn index(banks: &[(u16, &[[u8; 3]])]) -> Vec<u8> {
    let first_header = 0x58 + 16 * banks.len();
    let (mut table, mut headers) = (Vec::new(), Vec::new());
    for (number, &(phrase, takes)) in banks.iter().enumerate() {
        let mut header = Vec::new();
        header.extend_from_slice(&phrase.to_le_bytes());
        header.extend_from_slice(&0xFFFFu16.to_le_bytes());
        header.extend_from_slice(&[3, u8::try_from(takes.len()).unwrap(), 0, 0, 0, 0, 0, 0, 0, 0]);
        for (take, selectors) in takes.iter().enumerate() {
            header.extend_from_slice(selectors);
            if take + 1 < takes.len() {
                header.extend_from_slice(&u16::try_from(take + 1).unwrap().to_be_bytes());
            }
        }
        for word in [0x0100_0000 | number, header.len(), first_header + headers.len(), 0] {
            table.extend_from_slice(&u32::try_from(word).unwrap().to_le_bytes());
        }
        headers.extend_from_slice(&header);
    }
    let mut out = vec![0u8; 0x54];
    out[..4].copy_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&u32::try_from(banks.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&table);
    out.extend_from_slice(&headers);
    out
}

/// The install's three NIS records with a few takes of each bank, in the install's order of tests and selectors.
fn sample() -> (SpeechIndex, Sentences) {
    let start: &[(u32, u8)] = &[(0xF_FFFF, 1), (0xF_FFFF, 2), (0x7, 3)];
    let blacklist: &[(u32, u8)] = &[(0x7_FFFF, 3), (0xF_FFFF, 1), (0x7, 2)];
    let end: &[(u32, u8)] = &[(0x1F_FFFF, 1), (0xF_FFFF, 2), (0x7, 3)];
    let sentences = events(&[(0x4000, 0x4023, start), (0x4014, 0x4025, blacklist), (0x4011, 0x4024, end)]);
    let banks: [(u16, &[[u8; 3]]); 3] = [
        (0x4025, &[[1, 13, 0], [1, 13, 2], [5, 13, 0], [5, 13, 2]]),
        (0x4024, &[[7, 13, 0], [14, 13, 0], [0, 13, 0], [20, 13, 0]]),
        (0x4023, &[[14, 13, 0], [14, 13, 2], [18, 13, 0], [18, 0, 0], [4, 13, 0], [4, 13, 2]]),
    ];
    let index = SpeechIndex::parse(&index(&banks)).expect("the sample index parses");
    (index, Sentences::parse(&sentences).expect("the sample events parse"))
}

#[test]
fn an_arrest_plays_its_own_take_of_the_end_bank() {
    let (index, sentences) = sample();
    let take = |scene: &str| nis_take(&index, &sentences, scene).map(|t| (t.bank, t.take));
    assert_eq!(take("ArrestF02"), Some((1, 0)));
    assert_eq!(take("ArrestF02b"), Some((1, 0)));
    assert_eq!(take("ArrestF18"), Some((1, 1)));
    assert_eq!(take("ArrestM01b"), Some((1, 2)));
    assert_eq!(take("EndingNis04"), Some((1, 3)));
    assert_eq!(take("ArrestF06"), None, "no take of another scene stands in");
}

#[test]
fn the_start_and_blacklist_scenes_play_their_complete_takes() {
    let (index, sentences) = sample();
    let take = |scene: &str| nis_take(&index, &sentences, scene).map(|t| (t.bank, t.take));
    assert_eq!(take("IntroNisDD"), Some((2, 0)));
    assert_eq!(take("GenericStart"), Some((2, 2)), "track 0, not track 1");
    assert_eq!(take("IntroNis05"), Some((2, 4)));
    assert_eq!(take("IntroNisBL02"), Some((0, 0)));
    assert_eq!(take("IntroNisBL06End"), Some((0, 2)), "the complete take of its race");
    assert_eq!(take("Flythrough"), None);
    assert_eq!(take("FMVTest"), None);
}

#[test]
fn the_first_choice_is_the_first_option() {
    assert_eq!(First.below(7), 0);
    assert_eq!(First.below(0), 0);
}
