//! The air-support switches: the defaults, the file's keys by section, and which cues each switch gates.

use super::air_support::{AirSupport, parse};
use super::cues::Cue;

#[test]
fn an_empty_file_is_every_switch_on() {
    assert_eq!(parse(""), AirSupport::default());
    assert!(AirSupport::default().search_pattern);
    assert!(AirSupport::default().no_speech_dropoff);
}

#[test]
fn a_false_key_in_its_section_turns_only_its_switch_off() {
    let air = parse("[Pilot]\nSearchPattern = false\n");
    assert!(!air.search_pattern);
    assert!(air.ram_lines && air.weather_report && air.attack_missed);
    assert!(air.rain_flag && air.no_speech_dropoff);
}

#[test]
fn a_key_under_the_wrong_section_is_not_the_switch() {
    let air = parse("[Radio]\nSearchPattern = false\n");
    assert!(air.search_pattern, "SearchPattern lives under [Pilot]");
}

#[test]
fn the_radio_keys_are_read_from_the_radio_section() {
    let air = parse("[Radio]\nRainFlag = 0\nFastAirRequest = FALSE\n// a comment\n[Pilot]\nHoldOn = true\n");
    assert!(!air.rain_flag);
    assert!(!air.fast_air_request);
    assert!(air.fast_air_reminder && air.hold_on);
}

#[test]
fn a_later_line_overrides_an_earlier_one() {
    let air = parse("[Pilot]\nRamLines = false\nRamLines = true\n");
    assert!(air.ram_lines);
}

#[test]
fn a_malformed_value_is_on() {
    let air = parse("[Pilot]\nBeenHit = maybe\n");
    assert!(air.been_hit);
}

#[test]
fn the_base_chase_lines_are_never_switched_off() {
    let off = parse(
        "[Pilot]\nSearchPattern = false\nSpottedAgain = false\n[Radio]\nRainFlag = false\nNoSpeechDropoff = false\n",
    );
    assert!(off.allows(Cue::Lost));
    assert!(off.allows(Cue::Regained));
    assert!(off.allows(Cue::HeatJump));
}

#[test]
fn a_switched_off_search_pattern_suppresses_the_time_expired_line() {
    let off = parse("[Pilot]\nSearchPattern = false\n");
    assert!(!off.allows(Cue::TimeExpired));
    assert!(AirSupport::default().allows(Cue::TimeExpired));
}
