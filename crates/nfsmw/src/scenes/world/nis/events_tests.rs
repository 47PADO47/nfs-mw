use super::*;
use std::collections::BTreeMap;
use std::sync::Arc;

use blackbox_attrib::vlt_hash;
use blackbox_sequence::{Action, Engine, Event, Response, State, System};

/// The static data of an event: its id, then its words.
fn event(id: u32, args: &[u32]) -> Event {
    let mut words = vec![id];
    words.extend_from_slice(args);
    Event { id, data: words.iter().flat_map(|w| w.to_le_bytes()).collect() }
}

/// One engine named `Synthetic`, with one system that starts a one-action response on `START`. The action lasts 2 s and
/// fires its lists at 0 s (a call, id 13), 0.5 s (a flash of 1.5 s) and 1 s (a call, id 11).
fn sequence() -> Package {
    let mut states = BTreeMap::new();
    states.insert(
        20,
        State { filter: None, responses: BTreeMap::from([(START, Response { end_state: 20, action: 40 })]) },
    );
    let system = System { initial: 20, default: 20, states };
    let engine = Engine { name: "Synthetic".into(), name_hash: 77, systems: BTreeMap::from([(10, system)]) };
    let action =
        Action { name_hash: 40, duration: 2.0, tags: BTreeMap::new(), times: vec![(0.0, 101), (0.5, 102), (1.0, 103)] };
    let event_lists = BTreeMap::from([
        (101, vec![event(super::ids::CELL_CALL, &[13])]),
        (102, vec![event(super::ids::SCREEN_FLASH, &[1.5_f32.to_bits()])]),
        (103, vec![event(super::ids::CELL_CALL, &[11])]),
    ]);
    Package { engines: vec![engine], actions: BTreeMap::from([(40, action)]), event_lists, strings: BTreeMap::new() }
}

#[test]
fn the_start_stimulus_is_the_vault_hash_of_the_word() {
    assert_eq!(START, vlt_hash("START"));
    assert_eq!(START, 0xA6A5_6EA1);
}

#[test]
fn each_event_fires_once_at_its_time() {
    let mut events = NisEvents::start(Arc::new(sequence()), "Synthetic").expect("the sequence starts");
    assert_eq!(events.advance(0.0), [NisEvent::CellCall(13)], "the time-zero event fires on the first call");
    assert!(events.advance(0.25).is_empty());
    assert_eq!(events.advance(0.25), [NisEvent::ScreenFlash { length: 1.5 }], "the 0.5 s event fires at 0.5 s");
    assert_eq!(events.advance(0.5), [NisEvent::CellCall(11)], "the 1 s event fires at 1 s");
    assert!(events.advance(0.5).is_empty(), "nothing fires twice");
    assert!(events.advance(5.0).is_empty(), "nothing fires after the action ends");
}

#[test]
fn a_big_step_fires_every_event_it_passes_in_order() {
    let mut events = NisEvents::start(Arc::new(sequence()), "Synthetic").expect("the sequence starts");
    let fired = events.advance(1.2);
    assert_eq!(
        fired,
        [NisEvent::CellCall(13), NisEvent::ScreenFlash { length: 1.5 }, NisEvent::CellCall(11)],
        "all three fire in one frame, in time order"
    );
}

#[test]
fn a_package_with_no_engine_gives_no_sequence() {
    let package = Package::default();
    assert!(NisEvents::start(Arc::new(package), "Synthetic").is_none());
}

#[test]
fn an_engine_is_found_by_the_scene_name_without_regard_to_case() {
    let mut package = sequence();
    package.engines[0].name = "IntroNis05".into();
    assert!(NisEvents::start(Arc::new(package), "introNIS05").is_some());
}

#[test]
fn an_event_with_no_handler_is_not_returned() {
    let mut package = sequence();
    package.event_lists.insert(101, vec![event(0x1234_5678, &[1, 2])]);
    let mut events = NisEvents::start(Arc::new(package), "Synthetic").expect("the sequence starts");
    assert!(events.advance(0.0).is_empty());
    assert_eq!(events.advance(0.5), [NisEvent::ScreenFlash { length: 1.5 }]);
}

#[test]
fn an_overlay_reads_its_two_strings_from_the_string_table() {
    let mut package = sequence();
    package.strings.insert(0x6353_0002, "BUSTED_OVERLAY.fng".into());
    package.strings.insert(0x6353_0001, "FLASHTRIGGER".into());
    let overlay = event(super::ids::OVERLAY_MESSAGE, &[0x6353_0002, 0x6353_0001]);
    assert_eq!(
        decode(&package, &overlay),
        Some(NisEvent::Overlay { package: "BUSTED_OVERLAY.fng".into(), message: "FLASHTRIGGER".into() })
    );
}

#[test]
fn a_play_effect_reads_its_name_point_and_magnitude() {
    let mut package = sequence();
    package.strings.insert(0x6353_0005, "exp_lg2".into());
    let floats = [1.0_f32, 2.0, 3.0, 0.5, 0.0, 0.25].map(f32::to_bits);
    let mut args = vec![0x6353_0005];
    args.extend(floats);
    let effect = event(super::ids::PLAY_EFFECT, &args);
    assert_eq!(
        decode(&package, &effect),
        Some(NisEvent::PlayEffect { name: "exp_lg2".into(), position: [1.0, 2.0, 3.0], magnitude: [0.5, 0.0, 0.25] })
    );
}

#[test]
fn a_car_shake_and_a_cop_light_read_their_fields() {
    let package = sequence();
    let shake =
        event(super::ids::CAR_SHAKE, &[0.003_f32.to_bits(), 0.9_f32.to_bits(), 1.0_f32.to_bits(), 6.0_f32.to_bits()]);
    assert_eq!(decode(&package, &shake), Some(NisEvent::CarShake { shake: 0.003, rate: 0.9, ramp: 1.0, time: 6.0 }));
    assert_eq!(decode(&package, &event(super::ids::COP_LIGHTS, &[1])), Some(NisEvent::CopLights(true)));
    assert_eq!(
        decode(&package, &event(super::ids::LIGHTS, &[1, 0])),
        Some(NisEvent::Lights { cop: true, head: false })
    );
}

#[test]
fn a_short_or_nonfinite_event_is_malformed() {
    let package = sequence();
    assert_eq!(decode(&package, &event(super::ids::CELL_CALL, &[])), None, "the id is there but the call is not");
    let nan = event(super::ids::SCREEN_FLASH, &[f32::NAN.to_bits()]);
    assert_eq!(decode(&package, &nan), None);
}

#[test]
fn an_unknown_event_decodes_to_its_id() {
    let package = sequence();
    assert_eq!(decode(&package, &event(0x1234_5678, &[])), Some(NisEvent::Other(0x1234_5678)));
}
