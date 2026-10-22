//! The per-tick path does not allocate once it is warm: history, encode cache, snapshot writer, packet build and read, and
//! the client's view decoder. Eight clients and 300 entities, straight on byte buffers without links.
#![allow(unsafe_code)] // Test-only: a counting allocator around System proves the per-tick path does not allocate.

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use blackbox_net_core::MAX_DATAGRAM;
use blackbox_net_core::bits::{BitReader, BitWriter};
use blackbox_net_core::conn::{ConnConfig, ConnEvent, Connection};
use blackbox_net_core::replicate::{ClientState, ClientViews, EncodeCache, History, ReplicateError, Schema};

use common::{Scene, all_candidates, schema};

/// Allocations and reallocations made so far.
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

/// The system allocator, counting every allocation and reallocation.
struct Counting;

// SAFETY: every call is forwarded to `System` unchanged; the only addition is a relaxed counter increment.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Clients in the scene.
const CLIENTS: usize = 8;
/// Snapshot budget in bits: the 1100-byte LAN size less the header allowance.
const BUDGET_BITS: usize = 1100 * 8 - 64;
/// Ticks run before counting starts, and ticks counted.
const WARM_UP: u32 = 200;
const COUNTED: u32 = 200;
/// The application frame type of a snapshot.
const FRAME_SNAPSHOT: u8 = 0x02;

/// Everything one tick touches, owned in one place so the tick can be a plain function.
struct World {
    schema: Schema,
    scene: Scene,
    history: History,
    cache: EncodeCache,
    candidates: Vec<(u16, f32)>,
    states: Vec<ClientState>,
    views: Vec<ClientViews>,
    server_conns: Vec<Connection>,
    client_conns: Vec<Connection>,
    snapshot: [u8; 1100],
    out: [u8; MAX_DATAGRAM],
    ack: [u8; MAX_DATAGRAM],
}

impl World {
    /// The scene of the test: 8 clients, 300 entities, every client sees every entity.
    fn new() -> Self {
        let scene = Scene::new(0xA110C, CLIENTS as u16, 300 - CLIENTS as u16);
        Self {
            schema: schema(),
            history: History::new(64),
            cache: EncodeCache::new(),
            candidates: all_candidates(&scene),
            states: (0..CLIENTS).map(|_| ClientState::new()).collect(),
            views: (0..CLIENTS).map(|_| ClientViews::new()).collect(),
            server_conns: (0..CLIENTS)
                .map(|c| {
                    Connection::new(ConnConfig {
                        local_id: 2 * c as u16 + 1,
                        remote_id: 2 * c as u16 + 2,
                        ..ConnConfig::default()
                    })
                })
                .collect(),
            client_conns: (0..CLIENTS)
                .map(|c| {
                    Connection::new(ConnConfig {
                        local_id: 2 * c as u16 + 2,
                        remote_id: 2 * c as u16 + 1,
                        ..ConnConfig::default()
                    })
                })
                .collect(),
            scene,
            snapshot: [0; 1100],
            out: [0; MAX_DATAGRAM],
            ack: [0; MAX_DATAGRAM],
        }
    }

    /// One tick at 60 Hz: the history frame, then for every client a snapshot, its packet, its decode and its ack. Every
    /// snapshot is delivered by the ack of the same tick, so each one is written against the one before.
    fn tick(&mut self, tick: u32) {
        let now = u64::from(tick) * 1000 / 60;
        self.scene.fill(&self.schema, &mut self.history, tick);
        self.cache.reset(tick);
        for c in 0..CLIENTS {
            let stats = {
                let mut w = BitWriter::new(&mut self.snapshot);
                self.states[c]
                    .write_snapshot(
                        &self.schema,
                        &self.history,
                        &mut self.cache,
                        tick,
                        &self.candidates,
                        BUDGET_BITS,
                        &mut w,
                    )
                    .expect("the snapshot is written")
            };
            let bytes = stats.bits.div_ceil(8);
            let frames = [(FRAME_SNAPSHOT, &self.snapshot[..bytes])];
            let sent = self.server_conns[c].write_packet(now, &frames, &mut self.out).expect("the packet is built");

            let mut outcome: Option<Result<u32, ReplicateError>> = None;
            let schema = &self.schema;
            let views = &mut self.views[c];
            self.client_conns[c]
                .read_packet(now, &mut self.out[..sent.len], |frame_type, payload| {
                    if frame_type == FRAME_SNAPSHOT && outcome.is_none() {
                        outcome = Some(views.read_snapshot(schema, &mut BitReader::new(payload)));
                    }
                })
                .expect("the packet is read");
            assert_eq!(outcome, Some(Ok(tick)), "client {c} decodes tick {tick}");

            let ack = self.client_conns[c].write_packet(now, &[], &mut self.ack).expect("the ack is built");
            self.server_conns[c].read_packet(now, &mut self.ack[..ack.len], |_, _| {}).expect("the ack is read");
            while let Some(event) = self.server_conns[c].poll_event() {
                if event == ConnEvent::Delivered(sent.number) {
                    self.states[c].on_delivered(tick);
                }
            }
        }
    }
}

#[test]
fn the_per_tick_path_allocates_nothing_once_warm() {
    // The counter must see an ordinary allocation, or the zero below means nothing.
    let probe_before = ALLOCATIONS.load(Ordering::Relaxed);
    let probe = std::hint::black_box(Vec::<u8>::with_capacity(64));
    assert!(ALLOCATIONS.load(Ordering::Relaxed) > probe_before, "the counting allocator is active");
    drop(probe);

    let mut world = World::new();
    for tick in 0..WARM_UP {
        world.tick(tick);
    }
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    for tick in WARM_UP..WARM_UP + COUNTED {
        world.tick(tick);
    }
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before;
    println!("allocations over {COUNTED} warm ticks: {allocations}");
    assert_eq!(allocations, 0, "the per-tick path must not allocate once warm");
}
