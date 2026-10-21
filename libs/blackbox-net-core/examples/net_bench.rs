//! Encode time of the snapshot writer with and without the shared encode cache: 16 clients, 300 entities, 600 snapshot ticks
//! at 20 Hz. Each client acknowledges every snapshot at once, so each one is written against the one before.
//!
//! Run with `cargo run --release -p blackbox-net-core --example net_bench`.

#[path = "../tests/common/mod.rs"]
mod common;

use std::time::{Duration, Instant};

use blackbox_net_core::bits::BitWriter;
use blackbox_net_core::replicate::{ClientState, EncodeCache, History, Schema, SpatialHash};

use common::{CELL_M, Scene, TICK_HZ, build_spatial, relevant_candidates, schema};

/// Clients.
const CLIENTS: usize = 16;
/// Entities besides the players.
const OTHERS: u16 = 284;
/// Snapshot ticks measured.
const SNAPSHOTS: u32 = 600;
/// Simulation ticks between two snapshots (60 Hz simulation, 20 Hz snapshots).
const TICKS_PER_SNAPSHOT: u32 = 3;
/// The snapshot size of the LAN budget, in bytes.
const SNAPSHOT_BYTES: usize = 1100;
/// The budget of one snapshot in bits.
const BUDGET_BITS: usize = SNAPSHOT_BYTES * 8 - 64;
/// The scene seed; the same in both runs.
const SEED: u64 = 0x10_0AD;

/// The result of one run.
struct Run {
    encode: Duration,
    payload_bytes: u64,
    hits: u64,
    misses: u64,
}

/// Runs the scene once. With `shared` the cache holds every client's bodies of a tick; without it the cache is reset
/// before each client, so every client encodes its own bodies. Only the snapshot writes are timed.
fn run(shared: bool) -> Run {
    let schema: Schema = schema();
    let scene = Scene::new(SEED, CLIENTS as u16, OTHERS);
    let mut history = History::new(64);
    let mut cache = EncodeCache::new();
    let mut spatial = SpatialHash::new(CELL_M);
    let mut states: Vec<ClientState> = (0..CLIENTS).map(|_| ClientState::new()).collect();
    let mut positions = Vec::new();
    let mut scratch = Vec::new();
    let mut lists: Vec<Vec<(u16, f32)>> = vec![Vec::new(); CLIENTS];
    let mut buf = [0u8; SNAPSHOT_BYTES];
    let mut encode = Duration::ZERO;
    let mut payload_bytes = 0u64;
    for step in 0..SNAPSHOTS {
        let tick = step * TICKS_PER_SNAPSHOT;
        scene.fill(&schema, &mut history, tick);
        scene.positions_into(tick, &mut positions);
        build_spatial(&scene, &positions, &mut spatial);
        for (c, list) in lists.iter_mut().enumerate() {
            relevant_candidates(&scene, &positions, &spatial, c, &mut scratch, list);
        }

        let start = Instant::now();
        if shared {
            cache.reset(tick);
        }
        for (c, state) in states.iter_mut().enumerate() {
            if !shared {
                cache.reset(tick);
            }
            let stats = {
                let mut w = BitWriter::new(&mut buf);
                state.write_snapshot(&schema, &history, &mut cache, tick, &lists[c], BUDGET_BITS, &mut w)
            };
            let stats = stats.expect("the snapshot is written");
            payload_bytes += stats.bits.div_ceil(8) as u64;
        }
        encode += start.elapsed();

        for state in &mut states {
            state.on_delivered(tick);
        }
    }
    Run { encode, payload_bytes, hits: cache.hits(), misses: cache.misses() }
}

fn main() {
    // One untimed pass of each variant warms the buffers, then the timed pass.
    for shared in [true, false] {
        let _ = run(shared);
    }
    let shared = run(true);
    let unshared = run(false);
    assert_eq!(shared.payload_bytes, unshared.payload_bytes, "sharing the cache must not change the bytes");

    let ticks = f64::from(SNAPSHOTS);
    let shared_ms = shared.encode.as_secs_f64() * 1000.0 / ticks;
    let unshared_ms = unshared.encode.as_secs_f64() * 1000.0 / ticks;
    let lookups = shared.hits + shared.misses;
    let snapshots = u64::from(SNAPSHOTS) * CLIENTS as u64;
    let average_bytes = shared.payload_bytes as f64 / snapshots as f64;
    let per_second = average_bytes * f64::from(TICK_HZ) / f64::from(TICKS_PER_SNAPSHOT);

    println!(
        "net_bench: {CLIENTS} clients, {} entities, {SNAPSHOTS} snapshot ticks at 20 Hz",
        CLIENTS + usize::from(OTHERS)
    );
    println!("encode, shared cache:    {shared_ms:.4} ms per snapshot tick");
    println!("encode, no sharing:      {unshared_ms:.4} ms per snapshot tick");
    println!("speed-up from sharing:   {:.2}x", unshared_ms / shared_ms);
    println!(
        "cache hit rate (shared): {:.1} % ({} hits of {lookups} lookups)",
        100.0 * shared.hits as f64 / lookups as f64,
        shared.hits
    );
    println!("average snapshot payload: {average_bytes:.1} bytes per client");
    println!("bytes per second per client: {per_second:.0} (20 Hz)");
}
