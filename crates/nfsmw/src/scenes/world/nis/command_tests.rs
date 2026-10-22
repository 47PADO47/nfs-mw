use super::*;

#[test]
fn the_nis_words_parse_with_track_zero_as_the_default() {
    assert_eq!(parse(&["list"]), Ok(NisCommand::List));
    assert_eq!(parse(&["play", "IntroNis05"]), Ok(NisCommand::Play { scene: "IntroNis05", track: 0 }));
    assert_eq!(parse(&["play", "ArrestF02", "2"]), Ok(NisCommand::Play { scene: "ArrestF02", track: 2 }));
    assert_eq!(parse(&["stop"]), Ok(NisCommand::Stop));
    assert_eq!(parse(&["status"]), Ok(NisCommand::Status));
}

#[test]
fn a_bad_nis_command_says_what_it_takes() {
    let cases: [&[&str]; 6] = [
        &[],
        &["play"],
        &["play", "IntroNis05", "two"],
        &["play", "IntroNis05", "-1"],
        &["frobnicate"],
        &["list", "extra"],
    ];
    for args in cases {
        let err = parse(args).expect_err("not a command");
        assert!(err.contains("usage") || err.contains("not a track number"), "{args:?}: {err}");
    }
}
