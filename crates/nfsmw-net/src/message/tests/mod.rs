mod garbage;
mod limits;
mod samples;

use std::collections::HashSet;

use super::*;

fn bytes_of(message: &Message) -> Vec<u8> {
    let mut out = Vec::new();
    encode(message, &mut out).unwrap();
    out
}

#[test]
fn every_message_round_trips_canonically() {
    for message in samples::all() {
        let bytes = bytes_of(&message);
        let decoded = decode(&bytes).unwrap_or_else(|err| panic!("{:#06x}: {err}", message.type_id()));
        assert_eq!(decoded, message);
        assert_eq!(bytes_of(&decoded), bytes);
        assert_eq!(peek_type(&bytes), Some(message.type_id()));
    }
}

#[test]
fn the_samples_cover_the_catalogue_once_each() {
    let samples = samples::all();
    let ids: HashSet<u16> = samples.iter().map(Message::type_id).collect();
    assert_eq!(ids.len(), samples.len());
    assert_eq!(ids.len(), 61);
    for id in &ids {
        assert_eq!(Message::known_format(*id), Some(1));
    }
}

#[test]
fn type_numbers_sit_in_their_namespaces() {
    for message in samples::all() {
        let id = message.type_id();
        match message.channel() {
            Channel::Events => assert!((0x1000..0x8000).contains(&id), "{id:#06x}"),
            Channel::Control | Channel::Chat | Channel::Bulk => assert!(id < 0x1000, "{id:#06x}"),
        }
    }
}

#[test]
fn every_truncation_is_an_error() {
    for message in samples::all() {
        let bytes = bytes_of(&message);
        for len in 0..bytes.len() {
            assert!(decode(&bytes[..len]).is_err(), "{:#06x} cut at {len}", message.type_id());
        }
    }
}

#[test]
fn trailing_bytes_are_refused_at_the_known_format() {
    let mut bytes = bytes_of(&SetReady { ready: true }.into());
    bytes.push(0);
    assert_eq!(decode(&bytes), Err(DecodeError::TrailingBytes));
}

#[test]
fn a_newer_format_skips_unknown_trailing_fields() {
    let message: Message = StartAt { start_tick: 77 }.into();
    let mut bytes = bytes_of(&message);
    bytes[2] = 2;
    bytes.extend_from_slice(&[0xFF, 0x01, 0x02]);
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn an_older_format_and_an_unknown_type_are_reported() {
    let mut bytes = bytes_of(&SetReady { ready: true }.into());
    bytes[2] = 0;
    assert_eq!(decode(&bytes), Err(DecodeError::BadFormat { type_id: 0x0012, format: 0 }));
    let unknown = [0x34, 0x12, 1, 0];
    assert_eq!(decode(&unknown), Err(DecodeError::UnknownType(0x1234)));
    assert!(DecodeError::UnknownType(0x1234).is_skippable());
    assert!(!DecodeError::TrailingBytes.is_skippable());
    let modded = [0x00, 0x80, 1];
    assert_eq!(decode(&modded), Err(DecodeError::UnknownType(0x8000)));
}

#[test]
fn channels_follow_the_doc() {
    assert_eq!(
        Message::from(Auth {
            proto_minor: 0,
            sim_build: crate::version::SimBuild { bytes: [0; 8] },
            data_id: crate::version::DataId { bytes: [0; 16] },
            caps: 0,
            connect_token: None,
            resume: None,
        })
        .channel(),
        Channel::Control
    );
    assert_eq!(Message::from(QuickMessage { id: 1 }).channel(), Channel::Chat);
    assert_eq!(Message::from(BulkEnd { id: 1 }).channel(), Channel::Bulk);
    assert_eq!(Message::from(Repair { head: EventHead::default(), entity_id: 1 }).channel(), Channel::Events);
    assert_eq!(Channel::Bulk.index(), 3);
}

#[test]
fn the_invalid_message_window_closes_on_the_seventeenth() {
    let mut window = InvalidMessages::new();
    for i in 0..INVALID_LIMIT as u64 {
        assert!(!window.record(i * 100));
    }
    assert!(window.record(1_700));
    let mut slow = InvalidMessages::new();
    for i in 0..100u64 {
        assert!(!slow.record(i * 1_000));
    }
    assert!(slow.count() <= INVALID_LIMIT);
}

#[test]
fn settings_patch_applies_only_what_it_names() {
    let base = samples::settings();
    let patch = SettingsPatch { laps: Some(5), nitrous: Some(false), ..SettingsPatch::default() };
    let changed = base.apply(&patch);
    assert_eq!(changed.laps, 5);
    assert!(!changed.nitrous);
    assert_eq!(changed.name, base.name);
    assert_eq!(changed.allowed_cars, base.allowed_cars);
    assert!(changed.check().is_ok());
}

#[test]
fn session_phases_map_from_the_core_machine() {
    use blackbox_net_core::session::Phase;
    assert_eq!(SessionPhase::from(Phase::Countdown { start_tick: 9 }), SessionPhase::Countdown);
    assert_eq!(SessionPhase::from(Phase::Results), SessionPhase::Results);
    assert_eq!(Mode::from_code(9), None);
    assert_eq!(Mode::from_code(130), Some(Mode::Custom(130)));
}
