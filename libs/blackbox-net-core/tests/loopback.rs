//! Sixteen clients over lossy links for 30 seconds of simulated time. Every snapshot decodes against the view it names, and
//! the server's record of each client's view matches what the client decoded.

mod common;

use std::net::SocketAddr;

use blackbox_net_core::MAX_DATAGRAM;
use blackbox_net_core::bits::{BitReader, BitWriter};
use blackbox_net_core::conn::{ConnConfig, ConnEvent, Connection};
use blackbox_net_core::link::{Link, LossConfig, LossyLink, MemoryLink, MemoryNet};
use blackbox_net_core::replicate::{
    ClientState, ClientViews, EncodeCache, History, ReplicateError, SnapshotStats, SpatialHash,
};

use common::{CELL_M, Scene, TICK_HZ, build_spatial, relevant_candidates, schema};

/// Clients, one player each.
const CLIENTS: usize = 16;
/// Other entities besides the players.
const OTHERS: u16 = 284;
/// Simulated ticks: 30 seconds at 60 Hz.
const TICKS: u32 = 1800;
/// A snapshot is sent every this many ticks (20 Hz).
const SNAPSHOT_EVERY: u32 = 3;
/// The LAN snapshot size: the datagram payload of one snapshot frame.
const SNAPSHOT_BYTES: usize = 1100;
/// The budget of one snapshot in bits: the bytes less the header allowance.
const BUDGET_BITS: usize = SNAPSHOT_BYTES * 8 - 64;
/// The application frame type of a snapshot.
const FRAME_SNAPSHOT: u8 = 0x02;
/// The port of the server; client `c` listens on `CLIENT_PORT + c`.
const SERVER_PORT: u16 = 9000;
const CLIENT_PORT: u16 = 9001;

/// The server's address.
fn server_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], SERVER_PORT))
}

/// Client `index`'s address.
fn client_addr(index: usize) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], CLIENT_PORT + index as u16))
}

/// The client index of a datagram's source address, if it is one of the clients.
fn client_index(addr: SocketAddr) -> Option<usize> {
    let index = usize::from(addr.port().checked_sub(CLIENT_PORT)?);
    (index < CLIENTS).then_some(index)
}

/// The network-facing state of the server: one link, and per client a connection, a replication state and the packet
/// numbers it sent (with their snapshot ticks).
struct ServerSide {
    link: LossyLink<MemoryLink>,
    conns: Vec<Connection>,
    states: Vec<ClientState>,
    sent: Vec<Vec<(u32, u32)>>,
}

/// The network-facing state of one client.
struct ClientSide {
    link: LossyLink<MemoryLink>,
    conn: Connection,
    views: ClientViews,
    received: u32,
}

/// Totals over every snapshot the server wrote.
#[derive(Debug, Default)]
struct Totals {
    snapshots: u64,
    free_of_skips: u64,
    payload_bytes: u64,
    datagram_bytes: u64,
    full: u64,
    delta: u64,
    removed: u64,
    skipped: u64,
    missing_baseline: u64,
    entities_checked: u64,
    entities_unchecked: u64,
}

impl Totals {
    /// Adds one snapshot's stats.
    fn record(&mut self, stats: &SnapshotStats) {
        self.snapshots += 1;
        self.free_of_skips += u64::from(stats.skipped == 0);
        self.payload_bytes += stats.bits.div_ceil(8) as u64;
        self.full += u64::from(stats.full);
        self.delta += u64::from(stats.delta);
        self.removed += u64::from(stats.removed);
        self.skipped += u64::from(stats.skipped);
    }
}

/// Checks the client's view of `tick` against the server's record of it: the same entities, and each entity equal to the
/// value in the frame of the source tick the server recorded. A source frame that has left the history is counted, not
/// checked.
fn verify(client: &ClientViews, state: &ClientState, history: &History, tick: u32, totals: &mut Totals) {
    let view = client.view(tick).expect("the client keeps the view it decoded");
    let held = state.held(tick).expect("the server keeps the view it sent");
    assert_eq!(view.len(), held.len(), "entity count at tick {tick}");
    for &(id, source) in held {
        match history.frame(source) {
            Some(source_frame) => {
                assert_eq!(view.get(id), source_frame.get(id), "entity {id} at tick {tick} (source {source})");
                totals.entities_checked += 1;
            }
            None => totals.entities_unchecked += 1,
        }
    }
}

#[test]
fn sixteen_clients_over_lossy_links_stay_in_sync() {
    let schema = schema();
    let scene = Scene::new(0x10_0AD, CLIENTS as u16, OTHERS);
    let net = MemoryNet::new();
    let impairments =
        |seed: u64| LossConfig { loss: 0.02, latency_ms: 20, jitter_ms: 5, duplicate: 0.0, reorder: 0.0, seed };

    let mut server = ServerSide {
        link: LossyLink::new(net.endpoint(server_addr()), impairments(1)),
        conns: (0..CLIENTS)
            .map(|c| {
                Connection::new(ConnConfig {
                    local_id: 2 * c as u16 + 1,
                    remote_id: 2 * c as u16 + 2,
                    ..ConnConfig::default()
                })
            })
            .collect(),
        states: (0..CLIENTS).map(|_| ClientState::new()).collect(),
        sent: vec![Vec::new(); CLIENTS],
    };
    let mut clients: Vec<ClientSide> = (0..CLIENTS)
        .map(|c| ClientSide {
            link: LossyLink::new(net.endpoint(client_addr(c)), impairments(100 + c as u64)),
            conn: Connection::new(ConnConfig {
                local_id: 2 * c as u16 + 2,
                remote_id: 2 * c as u16 + 1,
                ..ConnConfig::default()
            }),
            views: ClientViews::new(),
            received: 0,
        })
        .collect();

    let mut history = History::new(64);
    let mut cache = EncodeCache::new();
    let mut spatial = SpatialHash::new(CELL_M);
    let mut positions = Vec::new();
    let mut scratch = Vec::new();
    let mut candidates: Vec<(u16, f32)> = Vec::new();
    let mut snapshot = [0u8; SNAPSHOT_BYTES];
    let mut out = [0u8; MAX_DATAGRAM];
    let mut dgram = [0u8; MAX_DATAGRAM];
    let mut totals = Totals::default();

    for tick in 0..TICKS {
        let now = u64::from(tick) * 1000 / u64::from(TICK_HZ);
        server.link.set_now(now);
        for client in &mut clients {
            client.link.set_now(now);
        }
        scene.fill(&schema, &mut history, tick);
        scene.positions_into(tick, &mut positions);

        if tick % SNAPSHOT_EVERY == 0 {
            cache.reset(tick);
            build_spatial(&scene, &positions, &mut spatial);
            for c in 0..CLIENTS {
                relevant_candidates(&scene, &positions, &spatial, c, &mut scratch, &mut candidates);
                let stats = {
                    let mut w = BitWriter::new(&mut snapshot);
                    server.states[c].write_snapshot(
                        &schema,
                        &history,
                        &mut cache,
                        tick,
                        &candidates,
                        BUDGET_BITS,
                        &mut w,
                    )
                }
                .expect("the snapshot is written");
                totals.record(&stats);
                let bytes = stats.bits.div_ceil(8);
                let frames = [(FRAME_SNAPSHOT, &snapshot[..bytes])];
                let sent = server.conns[c].write_packet(now, &frames, &mut out).expect("the packet is built");
                totals.datagram_bytes += sent.len as u64;
                server.link.send(client_addr(c), &out[..sent.len]).expect("the datagram is sent");
                server.sent[c].push((sent.number, tick));
            }
        }

        for (c, client) in clients.iter_mut().enumerate() {
            while let Some((_, len)) = client.link.recv(&mut dgram).expect("a datagram is received") {
                let mut outcome: Option<Result<u32, ReplicateError>> = None;
                let views = &mut client.views;
                client
                    .conn
                    .read_packet(now, &mut dgram[..len], |frame_type, payload| {
                        if frame_type == FRAME_SNAPSHOT && outcome.is_none() {
                            let mut r = BitReader::new(payload);
                            outcome = Some(views.read_snapshot(&schema, &mut r));
                        }
                    })
                    .expect("the packet is read");
                match outcome {
                    Some(Ok(decoded)) => {
                        client.received += 1;
                        verify(&client.views, &server.states[c], &history, decoded, &mut totals);
                    }
                    Some(Err(ReplicateError::MissingBaseline)) => {
                        totals.missing_baseline += 1;
                        server.states[c].reset();
                    }
                    Some(Err(err)) => panic!("client {c}: a snapshot failed to decode: {err}"),
                    None => {}
                }
            }
            let ack = client.conn.write_packet(now, &[], &mut out).expect("the ack is built");
            client.link.send(server_addr(), &out[..ack.len]).expect("the ack is sent");
        }

        while let Some((from, len)) = server.link.recv(&mut dgram).expect("a datagram is received") {
            let Some(c) = client_index(from) else { continue };
            server.conns[c].read_packet(now, &mut dgram[..len], |_, _| {}).expect("the ack is read");
            while let Some(event) = server.conns[c].poll_event() {
                let ConnEvent::Delivered(number) = event else { continue };
                let Ok(index) = server.sent[c].binary_search_by_key(&number, |entry| entry.0) else { continue };
                server.states[c].on_delivered(server.sent[c][index].1);
            }
        }
    }

    let seconds = f64::from(TICKS) / f64::from(TICK_HZ);
    let snapshots_per_client = u64::from(TICKS / SNAPSHOT_EVERY);
    let hits = cache.hits();
    let lookups = hits + cache.misses();
    println!("clients: {CLIENTS}, entities: {}, ticks: {TICKS} ({seconds} s)", scene.movers().len());
    println!("snapshots written: {} ({} per client)", totals.snapshots, snapshots_per_client);
    println!("received per client: {:?}", clients.iter().map(|client| client.received).collect::<Vec<_>>());
    println!("average snapshot payload: {:.1} bytes", totals.payload_bytes as f64 / totals.snapshots as f64);
    let per_client_rate = totals.datagram_bytes as f64 / CLIENTS as f64 / seconds;
    println!("bytes per second per client (datagrams): {per_client_rate:.0}");
    println!(
        "entries per snapshot: full {}, delta {}, removed {}, skipped {}",
        totals.full, totals.delta, totals.removed, totals.skipped
    );
    println!("cache hit rate: {:.1} % ({hits} hits of {lookups} lookups)", 100.0 * hits as f64 / lookups as f64);
    println!(
        "entities checked against history: {}, unchecked (source left history): {}, missing baselines: {}",
        totals.entities_checked, totals.entities_unchecked, totals.missing_baseline
    );

    for (c, client) in clients.iter().enumerate() {
        assert!(client.received > 0, "client {c} received no snapshot");
        assert!(
            u64::from(client.received) * 10 >= snapshots_per_client * 9,
            "client {c} received {} of {snapshots_per_client} snapshots",
            client.received
        );
    }
    assert_eq!(totals.missing_baseline, 0, "a baseline went missing");
    assert!(totals.entities_checked > 0, "the views were checked against the history");
    let free = totals.free_of_skips as f64 / totals.snapshots as f64;
    assert!(free >= 0.95, "only {:.1} % of snapshots carried every changed relevant entity", free * 100.0);
    assert!(per_client_rate < 32_000.0, "{per_client_rate:.0} bytes per second per client");
}
