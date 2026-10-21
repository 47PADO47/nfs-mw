//! The movie subtitle files of the install: 30 files, their records, and the labels they point at.

use blackbox_text::StringTable;
use blackbox_text::subtitles::{SubtitleTrack, parse, tutorial_line_hash, upper_label_hash};

use crate::install;

const CLEAR: u32 = 0x001A_20BA;

fn is_tutorial(name: &str) -> bool {
    name.ends_with("_tutorial")
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_subtitle_files_hold_the_spec_records_and_resolve_in_english() {
    let Some(dir) = install() else { return };
    let names = dir.files_in("SUBTITLES");
    assert_eq!(names.len(), 30);
    let english = StringTable::from_file(&dir.read("LANGUAGES/English.bin").unwrap()).expect("English.bin");
    let lookup = |h: u32| english.get(h);

    let (mut records, mut cues_total, mut clear_cues) = (0, 0, 0);
    let mut plain_labels = Vec::new();
    let (mut tutorial_a, mut tutorial_b, mut tutorial_cues) = (0, 0, 0);
    for name in &names {
        let bytes = dir.read(&format!("SUBTITLES/{name}")).unwrap();
        assert_eq!(bytes.len() % 8, 0, "{name}: the size is a whole number of records");
        let file_records = bytes.len() / 8;
        records += file_records;
        let last = &bytes[bytes.len() - 8..];
        assert_eq!(u16::from_le_bytes([last[0], last[1]]), 0xFFFF, "{name} ends with its terminator");

        let cues = parse(&bytes);
        assert_eq!(cues.len(), file_records - 1, "{name}: exactly one terminator, at the end");
        cues_total += cues.len();
        clear_cues += cues.iter().filter(|c| c.label == CLEAR).count();

        if is_tutorial(name) {
            for cue in cues.iter().filter(|c| c.label != CLEAR) {
                tutorial_cues += 1;
                assert!(english.get(cue.label).is_none(), "{name}: a base label is not a stored string");
                tutorial_a += english.get(tutorial_line_hash(cue.label, "_A")).is_some() as usize;
                tutorial_b += english.get(tutorial_line_hash(cue.label, "_B")).is_some() as usize;
            }
            continue;
        }
        plain_labels.extend(cues.iter().filter(|c| c.label != CLEAR).map(|c| c.label));

        // Every line shows the English text of its label, and every clear cue shows nothing.
        let track = SubtitleTrack::new(&cues, CLEAR, lookup);
        for cue in &cues {
            let expected = if cue.label == CLEAR { None } else { english.get(cue.label) };
            let shown = track.line_at(cue.start_secs());
            assert!(shown == expected.as_deref(), "{name} at {}s", cue.start_secs());
        }
    }

    assert_eq!(records, 317, "records, terminators included");
    assert_eq!(cues_total, 287);
    assert_eq!(clear_cues, 118);
    assert_eq!(plain_labels.len(), 159, "non-tutorial, non-clear cues");
    let unresolved = plain_labels.iter().filter(|h| english.get(**h).is_none()).count();
    assert_eq!(unresolved, 0, "every non-clear label resolves in English.bin");

    assert_eq!(tutorial_cues, 10);
    assert_eq!((tutorial_a, tutorial_b), (10, 9), "tollbooth_tutorial's second line has no _B");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_tutorial_base_hash_is_the_upper_case_name_hash() {
    let Some(dir) = install() else { return };
    let english = StringTable::from_file(&dir.read("LANGUAGES/English.bin").unwrap()).expect("English.bin");
    let base = upper_label_hash("TUTORIAL_BOUNTY_PAGE1");
    assert_eq!(base, 0x3C47_46A0);
    let line_a = english.get(tutorial_line_hash(base, "_A")).expect("the A line of the bounty tutorial");
    assert!(!line_a.is_empty());
    assert_eq!(english.get(upper_label_hash("TUTORIAL_BOUNTY_PAGE1_A")).as_deref(), Some(line_a.as_str()));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_tollbooth_tutorial_shows_line_b_only_on_its_first_cue() {
    let Some(dir) = install() else { return };
    let english = StringTable::from_file(&dir.read("LANGUAGES/English.bin").unwrap()).expect("English.bin");
    let cues = parse(&dir.read("SUBTITLES/tollbooth_tutorial").unwrap());
    let track = SubtitleTrack::with_tutorial_lines(&cues, CLEAR, |h| english.get(h));
    let (first, second) = track.lines_at(cues[0].start_secs());
    assert!(first.is_some() && second.is_some());
    let (second_first, second_second) = track.lines_at(cues[1].start_secs());
    assert!(second_first.is_some() && second_second.is_none());
}
