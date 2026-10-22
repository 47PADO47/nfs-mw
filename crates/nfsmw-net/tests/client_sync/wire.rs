//! A small datagram format for the test: the real input frame and own-car block, the real entity records, and a
//! plain envelope around them in place of the connection layer.

use blackbox_net_core::bits::{BitReader, BitWriter};
use blackbox_net_core::replicate::KindId;
use nfsmw_net::clock::{Ping, Pong};
use nfsmw_net::input::InputFrame;
use nfsmw_net::schema::{Decoded, GameSchema, OWN_BLOCK_BITS, OwnBlock, Record};
use nfsmw_sim::{EntityView, Tick};

const INPUT: u8 = 1;
const SNAPSHOT: u8 = 2;
const PING: u8 = 3;
const PONG: u8 = 4;
const OWN_BYTES: usize = OWN_BLOCK_BITS / 8;

/// A snapshot: the server tick, the acknowledged input tick, the own-car block and the other entities.
pub struct Snapshot {
    pub tick: Tick,
    pub ack_input: Tick,
    pub own: OwnBlock,
    pub entities: Vec<Decoded>,
}

pub enum Datagram {
    Input(InputFrame),
    Snapshot(Snapshot),
    Ping(Ping),
    Pong(Pong),
}

pub fn input(frame: &InputFrame) -> Vec<u8> {
    let mut buf = [0u8; 64];
    let len = frame.encode(&mut buf).expect("an input frame fits");
    let mut out = vec![INPUT];
    out.extend_from_slice(&buf[..len]);
    out
}

pub fn ping(ping: &Ping) -> Vec<u8> {
    let mut out = vec![PING, ping.seq];
    out.extend_from_slice(&ping.client_time_ms.to_le_bytes());
    out
}

pub fn pong(pong: &Pong) -> Vec<u8> {
    let mut out = vec![PONG, pong.seq];
    out.extend_from_slice(&pong.client_time_ms.to_le_bytes());
    out.extend_from_slice(&pong.server_recv_tick.to_le_bytes());
    out.extend_from_slice(&pong.server_send_tick.to_le_bytes());
    out
}

pub fn snapshot(schema: &GameSchema, tick: Tick, ack_input: Tick, own: &OwnBlock, others: &[EntityView]) -> Vec<u8> {
    let mut out = vec![SNAPSHOT];
    out.extend_from_slice(&tick.to_le_bytes());
    out.extend_from_slice(&ack_input.to_le_bytes());
    let mut block = [0u8; OWN_BYTES];
    let mut w = BitWriter::new(&mut block);
    schema.write_own(own, &mut w).expect("the own block encodes");
    out.extend_from_slice(&block);
    out.push(others.len() as u8);
    for view in others {
        let mut record = Record::default();
        schema.encode(view, None, &mut record).expect("the view encodes");
        out.extend_from_slice(&view.id.to_le_bytes());
        out.push(record.kind().0);
        out.push(record.values().len() as u8);
        for value in record.values() {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out
}

pub fn decode(schema: &GameSchema, bytes: &[u8]) -> Option<Datagram> {
    let (&kind, rest) = bytes.split_first()?;
    match kind {
        INPUT => InputFrame::decode(rest).ok().map(Datagram::Input),
        PING => Some(Datagram::Ping(Ping { seq: *rest.first()?, client_time_ms: u32_at(rest, 1)? })),
        PONG => Some(Datagram::Pong(Pong {
            seq: *rest.first()?,
            client_time_ms: u32_at(rest, 1)?,
            server_recv_tick: f64::from_le_bytes(rest.get(5..13)?.try_into().ok()?),
            server_send_tick: f64::from_le_bytes(rest.get(13..21)?.try_into().ok()?),
        })),
        SNAPSHOT => decode_snapshot(schema, rest).map(Datagram::Snapshot),
        _ => None,
    }
}

fn decode_snapshot(schema: &GameSchema, bytes: &[u8]) -> Option<Snapshot> {
    let tick = u32_at(bytes, 0)?;
    let ack_input = u32_at(bytes, 4)?;
    let own = schema.read_own(&mut BitReader::new(bytes.get(8..8 + OWN_BYTES)?)).ok()?;
    let mut at = 8 + OWN_BYTES;
    let count = *bytes.get(at)?;
    at += 1;
    let mut entities = Vec::new();
    for _ in 0..count {
        let id = u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?);
        let kind = KindId(*bytes.get(at + 2)?);
        let len = usize::from(*bytes.get(at + 3)?);
        at += 4;
        let values: Vec<u32> = (0..len).map(|i| u32_at(bytes, at + i * 4)).collect::<Option<_>>()?;
        at += len * 4;
        entities.push(schema.decode(id, kind, &values).ok()?);
    }
    Some(Snapshot { tick, ack_input, own, entities })
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}
