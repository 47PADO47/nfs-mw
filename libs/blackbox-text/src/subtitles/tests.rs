use super::*;
use blackbox_hash::bstring_hash;
use std::collections::HashMap;

/// The clear value used by these tests. The track takes it as a parameter, so any value works.
const CLEAR: u32 = 0x0000_C1EA;

/// Packs `(start, label)` records with a zero pad, then the terminator if `terminated`.
fn records(cues: &[(u16, u32)], terminated: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for (start, label) in cues {
        out.extend_from_slice(&start.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&label.to_le_bytes());
    }
    if terminated {
        out.extend_from_slice(&[0xFF, 0xFF, 0, 0]);
        out.extend_from_slice(&CLEAR.to_le_bytes());
    }
    out
}

fn cue(start_tenths: u16, label: u32) -> Cue {
    Cue { start_tenths, label }
}

#[test]
fn reads_each_record_in_file_order() {
    let bytes = records(&[(0, 7), (54, 9), (60, CLEAR)], true);
    assert_eq!(parse(&bytes), [cue(0, 7), cue(54, 9), cue(60, CLEAR)]);
}

#[test]
fn stops_at_the_terminator_and_ignores_what_follows() {
    let mut bytes = records(&[(10, 1)], true);
    bytes.extend_from_slice(&records(&[(20, 2)], false));
    assert_eq!(parse(&bytes), [cue(10, 1)]);
}

#[test]
fn a_file_without_terminator_ends_at_its_last_whole_record() {
    assert_eq!(parse(&records(&[(10, 1), (20, 2)], false)), [cue(10, 1), cue(20, 2)]);
}

#[test]
fn a_trailing_partial_record_is_ignored() {
    let mut bytes = records(&[(10, 1)], false);
    bytes.extend_from_slice(&[0x20, 0, 0, 0, 2]);
    assert_eq!(parse(&bytes), [cue(10, 1)]);
}

#[test]
fn the_padding_field_is_not_read() {
    let bytes = [0x0A, 0x00, 0x87, 0x00, 0x05, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0];
    assert_eq!(parse(&bytes), [cue(10, 5)]);
}

#[test]
fn empty_and_damaged_input_give_no_cues_and_no_panic() {
    assert!(parse(&[]).is_empty());
    assert!(parse(&[0xFF, 0xFF, 0, 0, 0, 0, 0, 0]).is_empty());
    assert!(parse(&[1, 2, 3]).is_empty());
    let good = records(&[(10, 1), (20, 2)], true);
    for cut in 0..good.len() {
        let _ = parse(&good[..cut]);
    }
}

#[test]
fn start_is_in_tenths_of_a_second() {
    assert!((cue(54, 0).start_secs() - 5.4).abs() < 1e-6);
    assert_eq!(cue(0, 0).start_secs(), 0.0);
}

#[test]
fn upper_label_hash_is_the_hash_of_the_upper_case_name() {
    assert_eq!(upper_label_hash("abc_1"), bstring_hash("ABC_1"));
    assert_eq!(upper_label_hash("HELLO"), bstring_hash("HELLO"));
    assert_ne!(upper_label_hash("abc_1"), bstring_hash("abc_1"));
}

#[test]
fn tutorial_line_hash_continues_from_the_base_hash() {
    let base = upper_label_hash("BASE_PAGE1");
    assert_eq!(tutorial_line_hash(base, "_A"), upper_label_hash("BASE_PAGE1_A"));
    assert_eq!(tutorial_line_hash(base, "_B"), upper_label_hash("BASE_PAGE1_B"));
}

/// Text for a fixed set of label hashes.
fn lookup(texts: &[(u32, &str)]) -> impl Fn(u32) -> Option<String> {
    let map: HashMap<u32, String> = texts.iter().map(|(h, t)| (*h, t.to_string())).collect();
    move |h| map.get(&h).cloned()
}

#[test]
fn an_ordinary_track_shows_the_last_cue_at_or_before_the_time() {
    let (one, two, three) = (upper_label_hash("ONE"), upper_label_hash("TWO"), upper_label_hash("THREE"));
    let cues = [cue(10, one), cue(30, two), cue(50, CLEAR), cue(70, three)];
    let track = SubtitleTrack::new(&cues, CLEAR, lookup(&[(one, "one"), (two, "two"), (three, "three")]));
    assert_eq!(track.line_at(0.99), None, "before the first cue");
    assert_eq!(track.line_at(1.0), Some("one"), "a cue shows from its own start");
    assert_eq!(track.line_at(2.9), Some("one"));
    assert_eq!(track.line_at(3.0), Some("two"));
    assert_eq!(track.line_at(5.0), None, "the clear cue");
    assert_eq!(track.line_at(6.9), None);
    assert_eq!(track.line_at(7.0), Some("three"));
    assert_eq!(track.line_at(1000.0), Some("three"), "the last cue holds");
}

#[test]
fn an_ordinary_track_has_only_line_one() {
    let label = upper_label_hash("ONE");
    let track = SubtitleTrack::new(&[cue(10, label)], CLEAR, lookup(&[(label, "one")]));
    assert_eq!(track.lines_at(1.0), (Some("one"), None));
}

#[test]
fn clear_is_the_caller_value_not_a_built_in_one() {
    let label = upper_label_hash("ONE");
    let cues = [cue(10, label), cue(20, 0x1234)];
    let track = SubtitleTrack::new(&cues, 0x1234, lookup(&[(label, "one"), (0x1234, "not a line")]));
    assert_eq!(track.line_at(1.0), Some("one"));
    assert_eq!(track.line_at(2.0), None, "the cue with the passed clear value clears");
}

#[test]
fn missing_and_empty_text_show_nothing() {
    let (empty, missing) = (upper_label_hash("EMPTY"), upper_label_hash("MISSING"));
    let cues = [cue(10, empty), cue(20, missing)];
    let track = SubtitleTrack::new(&cues, CLEAR, lookup(&[(empty, "")]));
    assert_eq!(track.line_at(1.0), None);
    assert_eq!(track.line_at(2.0), None);
}

#[test]
fn a_tutorial_cue_takes_its_two_lines_from_the_base_label() {
    let (base_a, base_b) = (upper_label_hash("TUT_PAGE1"), upper_label_hash("TUT_PAGE2"));
    let cues = [cue(5, base_a), cue(30, base_b), cue(60, CLEAR)];
    let texts = [
        (tutorial_line_hash(base_a, "_A"), "first a"),
        (tutorial_line_hash(base_a, "_B"), "first b"),
        (tutorial_line_hash(base_b, "_A"), "second a"),
    ];
    let track = SubtitleTrack::with_tutorial_lines(&cues, CLEAR, lookup(&texts));
    assert_eq!(track.lines_at(0.4), (None, None), "before the first cue");
    assert_eq!(track.lines_at(1.0), (Some("first a"), Some("first b")));
    assert_eq!(track.lines_at(3.5), (Some("second a"), None), "a missing line stays hidden");
    assert_eq!(track.lines_at(6.5), (None, None), "a clear cue");
}

#[test]
fn a_tutorial_cue_with_a_direct_label_shows_one_line() {
    let direct = upper_label_hash("DIRECT");
    let track = SubtitleTrack::with_tutorial_lines(&[cue(5, direct)], CLEAR, lookup(&[(direct, "direct")]));
    assert_eq!(track.lines_at(1.0), (Some("direct"), None));
}

#[test]
fn a_base_label_is_not_split_on_an_ordinary_track() {
    let base = upper_label_hash("TUT_PAGE1");
    let texts = [(tutorial_line_hash(base, "_A"), "a")];
    let track = SubtitleTrack::new(&[cue(5, base)], CLEAR, lookup(&texts));
    assert_eq!(track.lines_at(1.0), (None, None));
}

#[test]
fn a_tutorial_clear_cue_shows_nothing() {
    let track = SubtitleTrack::with_tutorial_lines(&[cue(5, CLEAR)], CLEAR, lookup(&[]));
    assert_eq!(track.lines_at(1.0), (None, None));
}
