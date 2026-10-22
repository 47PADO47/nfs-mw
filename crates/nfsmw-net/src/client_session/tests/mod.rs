//! A scripted server on the mock link: it reads what the session sent and answers with real encoded messages and
//! frames. Times are milliseconds; the server's tick is `now * 60 / 1000`.

mod admission;
mod events;
mod garbage;
mod input;
mod lobby;
mod reconnect;
mod snapshots;
mod spectator;

use blackbox_net_core::bits::{BitWriter, IdGapEncoder};
use blackbox_net_core::endpoint::ClientEvent;
use blackbox_net_core::replicate::{OP_DELTA, OP_FULL, OP_REMOVE, encode_delta, encode_full};
use blackbox_net_core::secure::KeyId;
use nfsmw_sim::toy::Car;
use nfsmw_sim::{EntityKind, EntityView, PlayerId, Tick};

use super::{ClientSession, MockLink, MockPeer, Sent, SessionConfig, SessionEvent, mock_link};
use crate::clock::Pong;
use crate::frames::{
    FRAME_PING, FRAME_PONG, FRAME_SNAPSHOT, GlobalBlock, SnapshotHeader, decode_ping, encode_pong, write_snapshot,
};
use crate::loadout::Loadout;
use crate::message::{EventHead, JoinAccepted, Message, Name, ServerCaps, ServerName, Settings, decode, encode};
use crate::predict::ToyCar;
use crate::schema::{GameSchema, OwnBlock, Record};
use crate::version::{DataId, SimBuild};

pub const SLOT: u8 = 2;
pub const ENTITY: u16 = 2;
pub const START_MS: u64 = 10_000;

pub type Session = ClientSession<MockLink, ToyCar>;

pub fn config() -> SessionConfig {
    SessionConfig::new(
        Name::new("tester").unwrap(),
        Loadout::stock(3),
        SimBuild::from_parts("abc", "test"),
        DataId::compute(&[1; 32], &[("a", 1)]),
    )
}

pub fn car() -> ToyCar {
    ToyCar::new(Car::new([0.0, 0.0], [0.0, 1.0], 1.0, 50.0), 500.0)
}

/// The server tick at `now_ms`.
pub fn server_tick(now_ms: u64) -> f64 {
    now_ms as f64 * 0.06
}

pub fn accepted(slot: u8, token: &[u8]) -> JoinAccepted {
    JoinAccepted {
        player_id: [slot; 16],
        slot,
        entity_id: u16::from(slot),
        session_seed: 7,
        tick: 600,
        snapshot_hz: 20,
        settings: Settings::lan(ServerName::new("room").unwrap()),
        world_half_extent: GameSchema::DEFAULT_HALF_EXTENT,
        resume_token: crate::message::Bytes::new(token.to_vec()).unwrap(),
        server_caps: ServerCaps { flags: 0, schema_id: GameSchema::new().schema_id() },
        proto_minor: 0,
    }
}

pub fn head(seq: u32, tick: u32, target_tick: u32) -> EventHead {
    EventHead { seq, tick, target_tick }
}

/// A player car at `x`, driven by `owner`.
pub fn car_view(id: u16, owner: Option<u8>, x: f32) -> EntityView {
    EntityView {
        id,
        kind: EntityKind::PlayerCar,
        owner: owner.map(PlayerId),
        position: [x, 0.0, 5.0],
        orientation: [0.0, 0.0, 0.0, 1.0],
        linear_velocity: [0.0; 3],
        angular_velocity: [0.0; 3],
        type_id: 3,
        flags: 0,
    }
}

/// One entry of a snapshot body.
pub enum Entry<'a> {
    Full(&'a EntityView),
    Delta { old: &'a EntityView, new: &'a EntityView },
    Remove(u16),
}

/// A replicate snapshot body: tick, distance, count, entries (ids ascending), zero padding.
pub fn body(schema: &GameSchema, tick: Tick, distance: u32, entries: &[Entry<'_>]) -> Vec<u8> {
    let mut buf = vec![0u8; 2048];
    let mut w = BitWriter::new(&mut buf);
    w.write_bits(u64::from(tick), 32).unwrap();
    w.write_varint(u64::from(distance)).unwrap();
    w.write_varint(entries.len() as u64).unwrap();
    let mut ids = IdGapEncoder::new();
    for entry in entries {
        match entry {
            Entry::Full(view) => {
                ids.write(&mut w, view.id).unwrap();
                w.write_bits(OP_FULL, 2).unwrap();
                let record = record(schema, view);
                encode_full(schema.schema(), record.kind(), record.values(), &mut w).unwrap();
            }
            Entry::Delta { old, new } => {
                ids.write(&mut w, new.id).unwrap();
                w.write_bits(OP_DELTA, 2).unwrap();
                let (old, new) = (record(schema, old), record(schema, new));
                assert!(encode_delta(schema.schema(), new.kind(), old.values(), new.values(), &mut w).unwrap());
            }
            Entry::Remove(id) => {
                ids.write(&mut w, *id).unwrap();
                w.write_bits(OP_REMOVE, 2).unwrap();
            }
        }
    }
    let len = w.bytes_written();
    buf.truncate(len);
    buf
}

fn record(schema: &GameSchema, view: &EntityView) -> Record {
    let mut record = Record::default();
    schema.encode(view, None, &mut record).unwrap();
    record
}

/// The own block of a car at rest at `x`.
pub fn own_block(x: f32) -> OwnBlock {
    OwnBlock { position: [x, 0.0, 0.0], orientation: [0.0, 0.0, 0.0, 1.0], ..OwnBlock::default() }
}

/// The scripted server and the session under test.
pub struct Harness {
    pub session: Session,
    pub peer: MockPeer,
    pub schema: GameSchema,
    pub now: u64,
}

impl Harness {
    /// A session over a fresh mock link, not yet connected.
    pub fn new(config: SessionConfig) -> Self {
        let (link, peer) = mock_link();
        Self { session: ClientSession::new(link, config, car()), peer, schema: GameSchema::new(), now: START_MS }
    }

    /// Connected and admitted in `SLOT`, events and sends drained.
    pub fn joined() -> Self {
        let mut harness = Self::new(config());
        harness.connect();
        harness.reliable(accepted(SLOT, b"token-1").into());
        harness.frame();
        harness.take_events();
        harness.sent();
        harness
    }

    /// Joined with the clock synced.
    pub fn synced() -> Self {
        let mut harness = Self::joined();
        harness.sync();
        harness
    }

    pub fn connect(&mut self) {
        self.peer.push(ClientEvent::Connected { server_key: KeyId([9; 16]) });
        self.frame();
    }

    /// One 16 ms frame.
    pub fn frame(&mut self) {
        self.now += 16;
        self.session.update(self.now, 16.0);
    }

    /// Frames until `ms` passed.
    pub fn run(&mut self, ms: u64) {
        let until = self.now + ms;
        while self.now < until {
            self.frame();
        }
    }

    pub fn reliable(&mut self, message: Message) {
        let mut bytes = Vec::new();
        encode(&message, &mut bytes).unwrap();
        self.peer.push(ClientEvent::Reliable { channel: message.channel().index(), bytes });
    }

    pub fn unreliable(&mut self, frame_type: u8, bytes: Vec<u8>) {
        self.peer.push(ClientEvent::Unreliable { frame_type, bytes });
    }

    pub fn sent(&mut self) -> Vec<Sent> {
        self.peer.take_sent()
    }

    /// The reliable messages the session sent since the last look.
    pub fn sent_messages(&mut self) -> Vec<Message> {
        self.sent()
            .into_iter()
            .filter_map(|sent| match sent {
                Sent::Reliable { bytes, .. } => Some(decode(&bytes).unwrap()),
                Sent::Unreliable { .. } => None,
            })
            .collect()
    }

    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        self.session.take_events()
    }

    /// Answers every PING the session sent, at the current server tick, for half a second.
    pub fn sync(&mut self) {
        for _ in 0..40 {
            self.frame();
            self.answer_pings();
        }
        assert!(self.session.server_tick().is_some(), "the clock syncs");
    }

    /// Answers the PINGs sent since the last look; returns the other sends.
    pub fn answer_pings(&mut self) -> Vec<Sent> {
        let mut rest = Vec::new();
        for sent in self.sent() {
            let Sent::Unreliable { frame_type: FRAME_PING, bytes, .. } = &sent else {
                rest.push(sent);
                continue;
            };
            let ping = decode_ping(bytes).unwrap();
            let tick = server_tick(self.now);
            let pong = Pong {
                seq: ping.seq,
                client_time_ms: ping.client_time_ms,
                server_recv_tick: tick,
                server_send_tick: tick,
            };
            self.unreliable(FRAME_PONG, encode_pong(&pong).unwrap().to_vec());
        }
        rest
    }

    /// A one-part snapshot (its tick is the body's) with an own block (when given) and a global block.
    pub fn snapshot(&mut self, _tick: Tick, ack: Tick, own: Option<&OwnBlock>, body: &[u8]) {
        let header = SnapshotHeader { part: 0, parts: 1, ack_input: ack as u16 };
        let mut out = Vec::new();
        write_snapshot(&self.schema, header, own, Some(&GlobalBlock::default()), body, &mut out).unwrap();
        self.unreliable(FRAME_SNAPSHOT, out);
    }
}
