//! The air-support switches: the defaults, the file's keys by section, and which cues each switch gates.

use super::air_support::{AirSupport, parse};
use super::cues::Cue;
use blackbox_pursuit::speech::Line;

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
        "[Pilot]
SearchPattern = false
SpottedAgain = false
[Radio]
RainFlag = false
NoSpeechDropoff = false
",
    );
    for line in [Line::LostVisual, Line::RegainVisual, Line::HeatJump, Line::AttemptVehicleStop, Line::DispBreakAway] {
        assert!(off.allows_line(line), "{line:?}");
    }
}

#[test]
fn a_switched_off_search_pattern_suppresses_the_time_expired_line() {
    let off = parse(
        "[Pilot]
SearchPattern = false
",
    );
    assert!(!off.allows_line(Line::DispTimeExpired));
    assert!(AirSupport::default().allows_line(Line::DispTimeExpired));
}

#[test]
fn each_air_support_switch_suppresses_only_its_own_line() {
    let cases = [
        (
            "[Pilot]
RoadblockCall = false
",
            Line::CallForRoadblock,
        ),
        (
            "[Pilot]
BeenHit = false
",
            Line::CollisionWorld,
        ),
        (
            "[Pilot]
BeenHit = false
",
            Line::CollWorldCivi,
        ),
        (
            "[Pilot]
Reacquire = false
",
            Line::HeliLostVisual,
        ),
        (
            "[Pilot]
SpottedAgain = false
",
            Line::Spotted,
        ),
        (
            "[Pilot]
KillShot = false
",
            Line::UnitDisabled,
        ),
        (
            "[Pilot]
RamLines = false
",
            Line::IntentToRam,
        ),
        (
            "[Pilot]
WeatherReport = false
",
            Line::WeatherReport,
        ),
    ];
    for (text, line) in cases {
        let air = parse(text);
        assert!(!air.allows_line(line), "{text:?} should switch off {line:?}");
        assert!(AirSupport::default().allows_line(line), "{line:?} is on by default");
    }
    assert!(
        !parse(
            "[Pilot]
WeatherReport = false
"
        )
        .allows(Cue::Rain),
        "the weather cue obeys its switch"
    );
}

#[test]
fn a_switched_off_air_support_line_leaves_the_others_on() {
    let air = parse(
        "[Pilot]
BeenHit = false
",
    );
    assert!(air.allows(Cue::Rain) && air.allows_line(Line::UnitDisabled) && air.allows_line(Line::HeliLostVisual));
}
