//! Each limit of protocol.md section 9 and `security-gameplay.md` 1.2: one at the limit passes, one over fails.

use super::{bytes_of, samples};
use crate::loadout::{Loadout, MAX_PARTS, PartChoice};
use crate::message::*;
use crate::version::{MAX_MODS, ModEntry, ModKind};

fn round_trip(message: Message) -> Result<Message, DecodeError> {
    decode(&bytes_of(&message))
}

#[test]
fn text_lengths() {
    assert!(ChatText::new(&"x".repeat(200)).is_ok());
    assert!(ChatText::new(&"x".repeat(201)).is_err());
    assert!(ChatText::new("   ").is_err());
    assert!(Name::new(&"é".repeat(24)).is_ok());
    assert!(Name::new(&"é".repeat(25)).is_err());
    assert_eq!(Name::new("  Razor ").unwrap().as_str(), "Razor");
    assert!(Name::new("bad\u{202E}name").is_err());
    assert!(Name::new("tab\tname").is_err());
    assert!(Plate::new("").is_ok());
}

#[test]
fn text_on_the_wire_must_be_clean() {
    let message: Message = QuickMessage { id: 1 }.into();
    assert!(round_trip(message).is_ok());
    // A ChatSend (All, no slot) with a hand-written text field.
    let send = |text: &[u8]| {
        let mut bytes = vec![0x00, 0x02, 1, 0, 0, text.len() as u8];
        bytes.extend_from_slice(text);
        decode(&bytes)
    };
    assert!(send(b"hello").is_ok());
    assert_eq!(send(b" hello"), Err(DecodeError::BadText));
    assert_eq!(send(b""), Err(DecodeError::BadText));
    assert_eq!(send(&[0xC3, 0x28]), Err(DecodeError::BadText));
    assert_eq!(send("a\u{2066}b".as_bytes()), Err(DecodeError::BadText));
    assert_eq!(send(b"a\x07b"), Err(DecodeError::BadText));
    // A compatibility form (a full-width letter, a ligature) is not NFKC: the wire refuses it.
    assert_eq!(send("\u{FF21}nn".as_bytes()), Err(DecodeError::BadText));
    assert_eq!(send("o\u{FB01}ce".as_bytes()), Err(DecodeError::BadText));
    assert!(send("caf\u{E9}".as_bytes()).is_ok(), "an NFKC letter with an accent is fine");
}

#[test]
fn text_is_made_normal_when_it_is_built_and_bounded_before() {
    assert_eq!(Name::new("\u{FF21}nn").unwrap().as_str(), "Ann");
    assert_eq!(Name::new("o\u{FB01}ce").unwrap().as_str(), "ofice");
    assert_eq!(Name::new("e\u{301}").unwrap().as_str(), "\u{E9}", "a letter and its accent compose");
    // U+FDFA is one character that becomes eighteen: the bound is checked on the normal form.
    assert!(Name::new("\u{FDFA}").is_ok());
    assert!(Name::new("\u{FDFA}\u{FDFA}").is_err());
    assert!(Name::new(&"\u{FDFA}".repeat(1000)).is_err(), "an input far past the bound is refused unprocessed");
}

#[test]
fn mod_manifest_limit() {
    let entry = |i: usize| ModEntry::new(&format!("mod{i}"), "1.0", [0; 32], ModKind::Code, false).unwrap();
    let full: Vec<ModEntry> = (0..MAX_MODS).map(entry).collect();
    assert!(List::<ModEntry, MAX_MODS>::new(full.clone()).is_ok());
    let mut over = full;
    over.push(entry(MAX_MODS));
    assert!(List::<ModEntry, MAX_MODS>::new(over).is_err());
}

#[test]
fn loadout_parts_limit() {
    let parts = |n: usize| (0..n).map(|i| PartChoice { slot: i as u16, part: 1 }).collect::<Vec<_>>();
    let mut loadout = Loadout::stock(1);
    loadout.parts = List::new(parts(MAX_PARTS)).unwrap();
    assert!(round_trip(SelectLoadout { loadout }.into()).is_ok());
    assert!(List::<PartChoice, MAX_PARTS>::new(parts(MAX_PARTS + 1)).is_err());
}

#[test]
fn whole_message_limit() {
    let mut big = vec![0x01, 0x03, 1];
    big.resize(MAX_MESSAGE_BYTES + 1, 0);
    assert_eq!(decode(&big), Err(DecodeError::TooLarge));
    let mut ok = vec![0x01, 0x03, 1];
    ok.resize(MAX_MESSAGE_BYTES, 0);
    assert_ne!(decode(&ok), Err(DecodeError::TooLarge));
}

#[test]
fn settings_limit_and_ranges() {
    let mut settings = samples::settings();
    assert!(settings.check().is_ok());
    settings.max_players = 17;
    assert_eq!(settings.check(), Err(DecodeError::BadValue));
    settings.max_players = 1;
    assert_eq!(settings.check(), Err(DecodeError::BadValue));
    settings.max_players = 16;
    settings.traffic_density = 101;
    assert_eq!(settings.check(), Err(DecodeError::BadValue));
    settings.traffic_density = 100;
    let mods: Vec<ModEntry> = (0..MAX_MODS)
        .map(|i| {
            ModEntry::new(&format!("a-rather-long-mod-identifier-{i}"), "10.20.30", [0; 32], ModKind::Code, false)
                .unwrap()
        })
        .collect();
    settings.required_mods = List::new(mods).unwrap();
    assert!(wire_len(&settings) > MAX_SETTINGS_BYTES);
    assert_eq!(settings.check(), Err(DecodeError::TooLarge));
    let bad = UpdateSettings { patch: SettingsPatch { max_players: Some(20), ..SettingsPatch::default() } };
    assert_eq!(round_trip(bad.into()), Err(DecodeError::BadValue));
}

#[test]
fn bulk_limits() {
    assert!(Bytes::<MAX_BULK_CHUNK>::new(vec![0; MAX_BULK_CHUNK]).is_ok());
    assert!(Bytes::<MAX_BULK_CHUNK>::new(vec![0; MAX_BULK_CHUNK + 1]).is_err());
    let start = |length| BulkStart { kind: BulkKind::ReplayChunk, id: 2, length, hash: [0; 32] };
    assert!(round_trip(start(MAX_BULK_TRANSFER).into()).is_ok());
    assert_eq!(round_trip(start(MAX_BULK_TRANSFER + 1).into()), Err(DecodeError::BadValue));
    let chunk = |offset| BulkChunk { id: 2, offset, bytes: Bytes::new(vec![1; 10]).unwrap() };
    assert!(round_trip(chunk(MAX_BULK_TRANSFER - 10).into()).is_ok());
    assert_eq!(round_trip(chunk(MAX_BULK_TRANSFER - 9).into()), Err(DecodeError::BadValue));
}

#[test]
fn slots_and_wheels_are_range_checked() {
    let joined =
        |slot| PlayerJoined { head: samples::head(1), slot, player_id: [0; 16], name: Name::new("A").unwrap() };
    assert!(round_trip(joined(31).into()).is_ok());
    assert_eq!(round_trip(joined(32).into()), Err(DecodeError::BadValue));
    let puncture = |wheel| Puncture { head: samples::head(1), entity_id: 1, wheel };
    assert!(round_trip(puncture(3).into()).is_ok());
    assert_eq!(round_trip(puncture(4).into()), Err(DecodeError::BadValue));
    let whisper = ChatSend { channel: ChatChannel::Whisper, to_slot: None, text: ChatText::new("hi").unwrap() };
    assert_eq!(round_trip(whisper.into()), Err(DecodeError::BadValue));
    let command = HostCommand { command: AdminCommand::Kick { slot: 40 } };
    assert_eq!(round_trip(command.into()), Err(DecodeError::BadValue));
}

#[test]
fn primitives_are_strict() {
    // SetReady with a bool byte of 2.
    assert_eq!(decode(&[0x12, 0x00, 1, 2]), Err(DecodeError::BadValue));
    // StartAt with a non-canonical varint (0 written in two bytes) and with a u32 overflow.
    assert_eq!(decode(&[0x1A, 0x00, 1, 0x80, 0x00]), Err(DecodeError::BadVarint));
    assert_eq!(decode(&[0x1A, 0x00, 1, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]), Err(DecodeError::BadValue));
    // Auth with a reserved capability bit.
    let mut auth = bytes_of(&samples::all()[0]);
    let caps_at = HEADER_LEN + 1 + 8 + 16;
    auth[caps_at] |= 0x80;
    assert_eq!(decode(&auth), Err(DecodeError::BadValue));
    // A NaN in a Respawn's yaw.
    let respawn: Message =
        Respawn { head: EventHead::default(), entity_id: 1, position: [0.0; 3], yaw: 0.0, cause: RespawnCause::Admin }
            .into();
    let mut bytes = bytes_of(&respawn);
    let yaw_at = bytes.len() - 5;
    bytes[yaw_at..yaw_at + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert_eq!(decode(&bytes), Err(DecodeError::BadValue));
    // An unknown enum tag (KickReason 9).
    assert_eq!(decode(&[0x08, 0x00, 1, 9, 0]), Err(DecodeError::BadEnum));
    // A reserved mode byte in a LoadWorld.
    assert_eq!(decode(&[0x18, 0x00, 1, 1, 9, 0, 0, 0, 0, 0, 0, 0, 0]), Err(DecodeError::BadEnum));
}

#[test]
fn a_room_password_is_the_same_secret_in_any_compatibility_form() {
    use super::super::password_secret;
    assert_eq!(password_secret("\u{FF50}ass word"), password_secret("pass word"));
    assert_eq!(password_secret(" pass "), b" pass ".to_vec(), "spaces are part of a password");
    assert_eq!(password_secret("e\u{301}"), password_secret("\u{E9}"));
}
