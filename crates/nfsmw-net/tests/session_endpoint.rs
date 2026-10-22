//! End to end over real sockets: a `ClientSession` on a `ClientEndpoint` joins a minimal scripted server on a
//! `ServerEndpoint` over loopback UDP (Noise handshake, sealed packets), syncs its clock, decodes snapshots, places
//! and predicts its car, sees a remote car and sends INPUT frames the server can decode. The real server loop is
//! `nfsmw-server`'s; this only plays its wire side.

use std::net::SocketAddr;
use std::thread;
use std::time::{Duration, Instant};

use blackbox_net_core::bits::{BitWriter, IdGapEncoder};
use blackbox_net_core::endpoint::{
    ClientEndpoint, ClientEndpointConfig, ServerEndpoint, ServerEndpointConfig, ServerEvent,
};
use blackbox_net_core::replicate::{OP_FULL, encode_full};
use blackbox_net_core::secure::{ClientConfig, IdentityKey, ServerConfig};
use blackbox_net_core::session::PeerId;
use blackbox_vehicle::InputState;
use nfsmw_net::client_session::{ClientSession, SessionConfig};
use nfsmw_net::clock::Pong;
use nfsmw_net::frames::{
    FRAME_PING, FRAME_PONG, FRAME_SNAPSHOT, GlobalBlock, SnapshotHeader, decode_ping, encode_pong, write_snapshot,
};
use nfsmw_net::input::{FRAME_INPUT, InputFrame};
use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::{Bytes, JoinAccepted, Message, Name, ServerCaps, ServerName, Settings, decode, encode};
use nfsmw_net::predict::ToyCar;
use nfsmw_net::schema::{GameSchema, OwnBlock, Record};
use nfsmw_net::version::{DataId, PROTO_MAJOR, SimBuild, noise_prologue};
use nfsmw_sim::toy::Car;
use nfsmw_sim::{EntityKind, EntityView, PlayerId};

const OWN: u16 = 1;
const OTHER: u16 = 5;

fn view(id: u16, x: f32) -> EntityView {
    EntityView {
        id,
        kind: EntityKind::PlayerCar,
        owner: Some(PlayerId(id as u8)),
        position: [x, 0.0, 0.0],
        orientation: [0.0, 0.0, 0.0, 1.0],
        linear_velocity: [0.0; 3],
        angular_velocity: [0.0; 3],
        type_id: 1,
        flags: 0,
    }
}

/// A full snapshot body of `views` (ids ascending).
fn body(schema: &GameSchema, tick: u32, views: &[EntityView]) -> Vec<u8> {
    let mut buf = vec![0u8; 1024];
    let mut w = BitWriter::new(&mut buf);
    w.write_bits(u64::from(tick), 32).unwrap();
    w.write_varint(0).unwrap();
    w.write_varint(views.len() as u64).unwrap();
    let mut ids = IdGapEncoder::new();
    for view in views {
        let mut record = Record::default();
        schema.encode(view, None, &mut record).unwrap();
        ids.write(&mut w, view.id).unwrap();
        w.write_bits(OP_FULL, 2).unwrap();
        encode_full(schema.schema(), record.kind(), record.values(), &mut w).unwrap();
    }
    let len = w.bytes_written();
    buf.truncate(len);
    buf
}

fn message(message: Message) -> Vec<u8> {
    let mut out = Vec::new();
    encode(&message, &mut out).unwrap();
    out
}

fn accepted(schema: &GameSchema) -> JoinAccepted {
    JoinAccepted {
        player_id: [1; 16],
        slot: 1,
        entity_id: OWN,
        session_seed: 1,
        tick: 0,
        snapshot_hz: 20,
        settings: Settings::lan(ServerName::new("loopback").unwrap()),
        world_half_extent: schema.half_extent(),
        resume_token: Bytes::new(vec![7; 16]).unwrap(),
        server_caps: ServerCaps { flags: 0, schema_id: schema.schema_id() },
        proto_minor: 0,
    }
}

#[test]
fn a_session_plays_over_loopback_udp() {
    let prologue = noise_prologue(PROTO_MAJOR);
    let server_identity = IdentityKey::from_secret(&[1; 32]).unwrap();
    let server_config = ServerEndpointConfig::lan(ServerConfig::lan(PROTO_MAJOR, prologue.as_bytes()), server_identity);
    let mut server = ServerEndpoint::bind(SocketAddr::from(([127, 0, 0, 1], 0)), None, server_config).unwrap();
    let client_config = ClientEndpointConfig::new(
        ClientConfig::lan(PROTO_MAJOR, prologue.as_bytes()),
        IdentityKey::from_secret(&[2; 32]).unwrap(),
    );
    let endpoint = ClientEndpoint::connect(server.local_addr(), client_config).unwrap();
    let config = SessionConfig::new(
        Name::new("looper").unwrap(),
        Loadout::stock(1),
        SimBuild::from_parts("t", "t"),
        DataId { bytes: [0; 16] },
    );
    let car = ToyCar::new(Car::new([0.0, 0.0], [0.0, 1.0], 1.0, 60.0), 500.0);
    let mut session = ClientSession::new(endpoint, config, car);

    let schema = GameSchema::new();
    let origin = Instant::now();
    let mut peer: Option<PeerId> = None;
    let mut joined = false;
    let mut last_snapshot_ms = 0;
    let mut inputs = 0;
    let mut display = Vec::new();
    let mut last_ms = 0;
    while origin.elapsed() < Duration::from_secs(20) {
        let now = origin.elapsed().as_millis() as u64;
        let server_tick = now as f64 * 0.06;
        session.update(now, (now - last_ms) as f32);
        last_ms = now;
        session.drive(|_| InputState { throttle: 0.5, ..InputState::default() });
        while let Some(event) = server.poll_event() {
            match event {
                ServerEvent::Connected { peer: who, .. } => peer = Some(who),
                ServerEvent::Reliable { peer: who, bytes, .. } => {
                    if let Ok(Message::Join(_)) = decode(&bytes) {
                        server.send_reliable(who, 0, message(accepted(&schema).into()));
                        joined = true;
                    }
                }
                ServerEvent::Unreliable { peer: who, frame_type: FRAME_PING, bytes } => {
                    let ping = decode_ping(&bytes).unwrap();
                    let pong = Pong {
                        seq: ping.seq,
                        client_time_ms: ping.client_time_ms,
                        server_recv_tick: server_tick,
                        server_send_tick: server_tick,
                    };
                    server.send_unreliable(who, FRAME_PONG, encode_pong(&pong).unwrap().to_vec(), None);
                }
                ServerEvent::Unreliable { frame_type: FRAME_INPUT, bytes, .. } => {
                    InputFrame::decode(&bytes).expect("the server reads the client's input");
                    inputs += 1;
                }
                _ => {}
            }
        }
        if let (Some(who), true) = (peer, joined)
            && now >= last_snapshot_ms + 50
        {
            last_snapshot_ms = now;
            let tick = server_tick as u32;
            let own = OwnBlock { position: [0.0; 3], orientation: [0.0, 0.0, 0.0, 1.0], ..OwnBlock::default() };
            let header = SnapshotHeader { part: 0, parts: 1, ack_input: tick as u16 };
            let views = [view(OWN, 0.0), view(OTHER, 10.0 + tick as f32 * 0.01)];
            let mut out = Vec::new();
            let global = GlobalBlock::default();
            write_snapshot(&schema, header, Some(&own), Some(&global), &body(&schema, tick, &views), &mut out).unwrap();
            server.send_unreliable(who, FRAME_SNAPSHOT, out, Some(tick));
        }
        session.remote_entities(&mut display);
        let sees_other = display.iter().any(|(id, _)| *id == OTHER);
        if session.own_pose().is_some() && sees_other && inputs >= 10 {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert!(session.status().is_joined(), "{:?}", session.status());
    assert!(session.own_pose().is_some(), "the own car is placed and predicted");
    assert!(display.iter().any(|(id, _)| *id == OTHER), "the remote car is drawn");
    assert!(inputs >= 10, "the server decoded {inputs} INPUT frames");
    assert_eq!(session.counters().bad_frames, 0);
    session.leave();
}
