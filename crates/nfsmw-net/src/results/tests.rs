use nfsmw_sim::{PlayerId, SimEvent};

use super::*;
use crate::message::{
    DecodeError, EventHead, List, MAX_MESSAGE_BYTES, Message, Mode, RESULT_BUSTED, RESULT_DNF, decode, encode,
};
use crate::modes::{CarSample, ModeInput};

fn p(id: u8) -> PlayerId {
    PlayerId(id)
}

fn row(slot: u8, place: u8) -> RecordRow {
    RecordRow {
        slot,
        entity_id: 100 + u16::from(slot),
        place,
        time_ms: 60_000 + u32::from(place),
        ..RecordRow::default()
    }
}

fn record(rows: Vec<RecordRow>) -> RaceRecord {
    RaceRecord {
        mode: Mode::Circuit,
        track: 0x1234,
        laps: 3,
        start_tick: 600,
        end_tick: 9_000,
        rows: List::new(rows).expect("within the limit"),
    }
}

fn encoded(record: &RaceRecord) -> Vec<u8> {
    let mut bytes = Vec::new();
    record.encode(&mut bytes);
    bytes
}

#[test]
fn a_record_round_trips() {
    let mut first = row(3, 1);
    first.points = 123.5;
    first.top_speed_cms = 8_250;
    first.flags = FLAG_BUSTED;
    let original = record(vec![first, row(0, 2), RecordRow { place: 0, ..row(9, 0) }]);
    let bytes = encoded(&original);
    assert_eq!(&bytes[..5], b"NMRR\x01");
    assert_eq!(RaceRecord::decode(&bytes), Ok(original));
}

#[test]
fn a_full_record_fits_one_message() {
    let rows = (0..MAX_RECORD_ROWS as u8).map(|slot| row(slot, slot + 1)).collect();
    let full = record(rows);
    assert_eq!(RaceRecord::decode(&encoded(&full)), Ok(full.clone()));
    let mut bytes = Vec::new();
    encode(&Message::RaceResults(full.to_message(EventHead::default())), &mut bytes).expect("fits");
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
}

#[test]
fn a_newer_format_skips_its_extra_fields_and_this_format_refuses_them() {
    let mut bytes = encoded(&record(vec![row(1, 1)]));
    bytes.extend_from_slice(&[7, 7, 7]);
    assert_eq!(RaceRecord::decode(&bytes), Err(RecordError::Decode(DecodeError::TrailingBytes)));
    bytes[4] = RECORD_FORMAT + 1;
    assert_eq!(RaceRecord::decode(&bytes), Ok(record(vec![row(1, 1)])));
    bytes[4] = 0;
    assert_eq!(RaceRecord::decode(&bytes), Err(RecordError::BadFormat(0)));
}

#[test]
fn damaged_records_are_refused_without_a_panic() {
    let bytes = encoded(&record(vec![row(1, 1), row(2, 2)]));
    assert_eq!(RaceRecord::decode(b"NMR"), Err(RecordError::BadMagic));
    assert_eq!(RaceRecord::decode(b"XXXX\x01"), Err(RecordError::BadMagic));
    for cut in 5..bytes.len() {
        assert!(RaceRecord::decode(&bytes[..cut]).is_err(), "cut at {cut}");
    }
    let mut flipped = bytes.clone();
    for index in 5..flipped.len() {
        flipped[index] ^= 0xFF;
        let _ = RaceRecord::decode(&flipped);
        flipped[index] ^= 0xFF;
    }
}

#[test]
fn duplicate_slots_or_places_and_a_backwards_end_are_refused() {
    let twice = record(vec![row(1, 1), row(1, 2)]);
    assert_eq!(RaceRecord::decode(&encoded(&twice)), Err(RecordError::Decode(DecodeError::BadValue)));
    let tie = record(vec![row(1, 1), row(2, 1)]);
    assert_eq!(tie.check(), Err(DecodeError::BadValue));
    let unranked = record(vec![row(1, 0), row(2, 0)]);
    assert_eq!(unranked.check(), Ok(()), "unranked rows share place 0");
    let backwards = RaceRecord { end_tick: 10, ..record(vec![]) };
    assert_eq!(backwards.check(), Err(DecodeError::BadValue));
}

#[test]
fn the_message_folds_record_flags_into_the_wire_bits() {
    let rows = vec![
        RecordRow { flags: 0, ..row(0, 1) },
        RecordRow { flags: FLAG_KNOCKED_OUT | FLAG_DNF, ..row(1, 2) },
        RecordRow { flags: FLAG_LEFT, ..row(2, 3) },
        RecordRow { flags: FLAG_BUSTED | FLAG_DNF, ..row(3, 4) },
    ];
    let head = EventHead { seq: 4, tick: 9_000, target_tick: 0 };
    let message = record(rows).to_message(head);
    assert_eq!(message.head, head);
    let flags: Vec<u8> = message.rows.iter().map(|row| row.flags).collect();
    assert_eq!(flags, [0, RESULT_DNF, RESULT_DNF, RESULT_BUSTED | RESULT_DNF]);
    let mut bytes = Vec::new();
    let wrapped = Message::RaceResults(message);
    encode(&wrapped, &mut bytes).expect("encodes");
    assert_eq!(decode(&bytes), Ok(wrapped));
}

#[test]
fn the_tally_counts_speed_damage_busts_pursuit_time_and_resets() {
    let mut tally = Tally::new();
    let fast = CarSample { player: p(1), velocity: [30.0, 0.0, 40.0], damage: 0.4, ..CarSample::default() };
    tally.feed(&ModeInput::Car(fast));
    tally.feed(&ModeInput::Car(CarSample { player: p(1), velocity: [10.0, 0.0, 0.0], damage: 2.0, ..fast }));
    tally.feed(&ModeInput::Sim(SimEvent::PursuitStarted { player: p(1) }));
    for _ in 0..90 {
        tally.end_tick();
    }
    tally.feed(&ModeInput::Sim(SimEvent::Busted { player: p(1) }));
    tally.feed(&ModeInput::Sim(SimEvent::PursuitEnded { player: p(1) }));
    tally.end_tick();
    tally.feed(&ModeInput::Reset { player: p(1), penalty_ms: 0 });
    let stats = tally.get(p(1)).expect("seen");
    assert_eq!(stats.top_speed, 50.0);
    assert_eq!(stats.damage, 1.0, "clamped to 1");
    assert_eq!((stats.busted, stats.resets, stats.pursuit_ticks), (1, 1, 90));
    assert_eq!(stats.pursuit_ms(), 1_500);
    assert!(!stats.in_pursuit());
    assert!(tally.get(p(2)).is_none());
}

#[test]
fn a_departure_stops_the_pursuit_clock() {
    let mut tally = Tally::new();
    tally.feed(&ModeInput::Sim(SimEvent::PursuitStarted { player: p(3) }));
    tally.end_tick();
    tally.feed(&ModeInput::Left { player: p(3) });
    tally.end_tick();
    assert_eq!(tally.get(p(3)).map(|stats| stats.pursuit_ticks), Some(1));
}

#[test]
fn build_record_joins_placings_stats_and_seats() {
    let mut roster = Roster::new();
    roster.insert(Seat { player: p(1), slot: 4, entity: 41 });
    roster.insert(Seat { player: p(2), slot: 7, entity: 72 });
    let mut tally = Tally::new();
    tally.feed(&ModeInput::Car(CarSample {
        player: p(2),
        velocity: [12.34, 0.0, 0.0],
        damage: 0.5,
        ..Default::default()
    }));
    let placings = [
        Placing { player: p(2), place: 1, time_ms: Some(50_000), best_lap_ms: Some(16_000), ..Placing::default() },
        Placing { player: p(1), place: 2, flags: FLAG_LEFT | FLAG_DNF, points: f32::NAN, ..Placing::default() },
        Placing { player: p(9), place: 3, ..Placing::default() },
    ];
    let meta = RecordMeta { mode: Mode::Sprint, track: 5, laps: 0, start_tick: 100, end_tick: 50 };
    let record = build_record(meta, &placings, &tally, &roster);
    assert_eq!(record.end_tick, 100, "a backwards end is clamped");
    assert_eq!(record.rows.len(), 2, "player 9 has no seat");
    let winner = record.rows[0];
    assert_eq!((winner.slot, winner.entity_id, winner.time_ms, winner.best_lap_ms), (7, 72, 50_000, 16_000));
    assert_eq!((winner.top_speed_cms, winner.damage), (1_234, 128));
    let left = record.rows[1];
    assert_eq!((left.slot, left.time_ms, left.points, left.flags), (4, 0, 0.0, FLAG_LEFT | FLAG_DNF));
    assert_eq!(RaceRecord::decode(&encoded(&record)), Ok(record));
}

#[test]
fn the_roster_refuses_a_taken_or_invalid_slot() {
    let mut roster = Roster::new();
    assert!(roster.insert(Seat { player: p(1), slot: 0, entity: 1 }));
    assert!(!roster.insert(Seat { player: p(2), slot: 0, entity: 2 }), "slot 0 is taken");
    assert!(!roster.insert(Seat { player: p(2), slot: 32, entity: 2 }), "outside the id space");
    assert!(roster.insert(Seat { player: p(1), slot: 3, entity: 9 }), "a player can move");
    assert_eq!(roster.player_of(9), Some(p(1)));
    roster.remove(p(1));
    assert!(roster.get(p(1)).is_none());
}
