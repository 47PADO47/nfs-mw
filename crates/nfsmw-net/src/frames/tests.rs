use blackbox_net_core::bits::{BitReader, BitWriter};
use blackbox_net_core::rng::SplitMix64;

use super::*;
use crate::clock::{Ping, Pong};
use crate::message::SessionPhase;
use crate::schema::{CorrectionCause, GameSchema, InputReport, OwnBlock};

fn own_block() -> OwnBlock {
    OwnBlock {
        position: [10.0, 0.0, -20.0],
        orientation: [0.0, 0.0, 0.0, 1.0],
        linear_velocity: [3.0, 0.0, 1.0],
        cause: CorrectionCause::Teleport,
        report: InputReport { early: 30, earliness: 2, ..InputReport::default() },
        ..OwnBlock::default()
    }
}

#[test]
fn ping_and_pong_round_trip() {
    let ping = Ping { seq: 7, client_time_ms: 0xDEAD_BEEF };
    assert_eq!(decode_ping(&encode_ping(&ping)), Ok(ping));
    let pong = Pong { seq: 9, client_time_ms: 1234, server_recv_tick: 100.5, server_send_tick: 101.25 };
    let bytes = encode_pong(&pong).expect("encodes");
    assert_eq!(decode_pong(&bytes), Ok(pong));
    assert_eq!(decode_ping(&[1, 2, 3]), Err(FrameError::BadLength));
    assert_eq!(decode_pong(&bytes[..14]), Err(FrameError::BadLength));
    let bad = Pong { server_recv_tick: f64::NAN, ..pong };
    assert_eq!(encode_pong(&bad), Err(FrameError::BadValue));
    let negative = Pong { server_send_tick: -1.0, ..pong };
    assert_eq!(encode_pong(&negative), Err(FrameError::BadValue));
}

#[test]
fn global_block_round_trips_and_refuses_reserved_bits() {
    let block = GlobalBlock {
        phase: SessionPhase::Running,
        heat: 4,
        busted: 0.5,
        cooldown: 1.0,
        pursuit: PursuitStatus::Chased,
        place: 3,
        lap: 2,
        checkpoint: 17,
        time_of_day: 600.0,
    };
    let mut bytes = [0u8; GLOBAL_BLOCK_BITS / 8];
    block.write(&mut BitWriter::new(&mut bytes)).expect("writes");
    let back = GlobalBlock::read(&mut BitReader::new(&bytes)).expect("reads");
    assert_eq!(back.phase, block.phase);
    assert_eq!((back.heat, back.place, back.lap, back.checkpoint), (4, 3, 2, 17));
    assert!((back.busted - 0.5).abs() < 0.01 && back.cooldown == 1.0);
    assert!((back.time_of_day - 600.0).abs() < 0.05);
    bytes[11] |= 1;
    assert_eq!(GlobalBlock::read(&mut BitReader::new(&bytes)), Err(FrameError::Reserved));
    bytes[0] = 0xF0;
    assert_eq!(GlobalBlock::read(&mut BitReader::new(&bytes)), Err(FrameError::BadBlock));
}

#[test]
fn snapshot_parts_round_trip() {
    let schema = GameSchema::new();
    let body = [0, 0, 1, 0, 0, 0];
    let header = SnapshotHeader { part: 0, parts: 2, ack_input: 250 };
    let global = GlobalBlock::default();
    let mut out = Vec::new();
    write_snapshot(&schema, header, Some(&own_block()), Some(&global), &body, &mut out).expect("writes");
    let frame = read_snapshot(&schema, &out).expect("reads");
    assert_eq!(frame.header, header);
    assert_eq!(frame.tick, 256);
    assert_eq!(frame.body, &body);
    assert_eq!(frame.own.map(|own| own.cause), Some(CorrectionCause::Teleport));
    assert_eq!(frame.global, Some(global));

    let second = SnapshotHeader { part: 1, ..header };
    assert_eq!(write_snapshot(&schema, second, Some(&own_block()), None, &body, &mut out), Err(FrameError::BadPart));
    write_snapshot(&schema, second, None, None, &body, &mut out).expect("a bare part writes");
    let frame = read_snapshot(&schema, &out).expect("reads");
    assert_eq!((frame.own, frame.global), (None, None));
    out[1] = 1;
    assert_eq!(read_snapshot(&schema, &out), Err(FrameError::BadPart));
    out[1] = 0x80;
    assert_eq!(read_snapshot(&schema, &out), Err(FrameError::Reserved));
    out[0] = 0x22;
    assert_eq!(read_snapshot(&schema, &out), Err(FrameError::BadPart));
}

#[test]
fn garbage_never_panics() {
    let schema = GameSchema::new();
    let mut rng = SplitMix64(0x5EED);
    for _ in 0..20_000 {
        let len = (rng.next_u64() % 80) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        let _ = read_snapshot(&schema, &bytes);
        let _ = decode_pong(&bytes);
        let _ = decode_ping(&bytes);
        let _ = GlobalBlock::read(&mut BitReader::new(&bytes));
    }
}
