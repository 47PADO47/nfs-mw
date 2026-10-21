//! The server and client halves together: round trips over many ticks with moving, appearing and disappearing entities,
//! forced full records when a source frame leaves the history, resets, and a baseline 64 ticks back.

use std::collections::BTreeMap;

use crate::bits::{BitReader, BitWriter};
use crate::replicate::{ClientState, ClientViews, EncodeCache, FieldDesc, History, KindId, Schema, SnapshotStats};
use crate::rng::SplitMix64;

/// The entities of a test scene: id to kind and field values.
type Entities = BTreeMap<u16, (KindId, Vec<u32>)>;

/// Bits of the whole snapshot buffer in the round-trip tests: far above what the scenes need, so nothing is skipped.
const ROOMY_BITS: usize = 4096 * 8;

/// A schema for the tests: kind 0 has a 16-bit diff field and a 10-bit raw field, kind 1 a 12-bit diff field and a 4-bit
/// raw field.
pub(super) fn round_schema() -> Schema {
    let mut schema = Schema::new();
    let first = schema.add_kind(&[FieldDesc { bits: 16, diff: true }, FieldDesc { bits: 10, diff: false }]);
    assert_eq!(first, Ok(KindId(0)));
    let second = schema.add_kind(&[FieldDesc { bits: 12, diff: true }, FieldDesc { bits: 4, diff: false }]);
    assert_eq!(second, Ok(KindId(1)));
    schema
}

/// Fresh values for a kind: a random position-like field and a random raw field.
fn fresh_values(rng: &mut SplitMix64, kind: KindId) -> Vec<u32> {
    match kind.0 {
        0 => vec![rng.below(65_536) as u32, rng.below(1024) as u32],
        _ => vec![rng.below(4096) as u32, rng.below(16) as u32],
    }
}

/// Moves every entity a little, sometimes re-rolls a raw field, and adds and removes entities at random. An id can come
/// back with the other kind, which exercises a full record in place of a delta.
pub(super) fn step(rng: &mut SplitMix64, entities: &mut Entities, ids: u16) {
    let keys: Vec<u16> = entities.keys().copied().collect();
    for id in keys {
        if rng.below(40) == 0 {
            entities.remove(&id);
        }
    }
    if rng.below(3) == 0 {
        let id = rng.below(u64::from(ids)) as u16;
        let kind = KindId(rng.below(2) as u8);
        entities.insert(id, (kind, fresh_values(rng, kind)));
    }
    for (kind, values) in entities.values_mut() {
        match kind.0 {
            0 => {
                let step = rng.below(41) as i64 - 20;
                values[0] = (i64::from(values[0]) + step).clamp(0, 65_535) as u32;
                if rng.below(4) == 0 {
                    values[1] = rng.below(1024) as u32;
                }
            }
            _ => {
                values[0] = (values[0] + rng.below(9) as u32) % 4096;
                if rng.below(3) == 0 {
                    values[1] = rng.below(16) as u32;
                }
            }
        }
    }
}

/// Writes `entities` into the history frame of `tick`.
pub(super) fn fill(schema: &Schema, history: &mut History, tick: u32, entities: &Entities) {
    let frame = history.begin(tick);
    for (&id, (kind, values)) in entities {
        let pushed = frame.push(schema, id, *kind, values);
        assert!(pushed.is_ok());
    }
}

/// Every candidate of `entities` with weight 1, sorted by id.
pub(super) fn all_candidates(entities: &Entities) -> Vec<(u16, f32)> {
    entities.keys().map(|&id| (id, 1.0)).collect()
}

/// Writes one snapshot of `tick` for `server` into `buf` and returns its stats and byte length.
#[allow(clippy::too_many_arguments)]
pub(super) fn snapshot(
    schema: &Schema,
    history: &History,
    cache: &mut EncodeCache,
    server: &mut ClientState,
    tick: u32,
    candidates: &[(u16, f32)],
    budget_bits: usize,
    buf: &mut [u8],
) -> (SnapshotStats, usize) {
    let stats = {
        let mut w = BitWriter::new(buf);
        server.write_snapshot(schema, history, cache, tick, candidates, budget_bits, &mut w)
    };
    let stats = stats.expect("the snapshot is written");
    (stats, stats.bits.div_ceil(8))
}

/// Checks that the client's view of `tick` holds exactly the server's recorded view: the same ids, and each entity's
/// values equal those of the source frame the server recorded for it.
pub(super) fn check_view(client: &ClientViews, server: &ClientState, history: &History, tick: u32) {
    let view = client.view(tick).expect("the client keeps the view");
    let held = server.held(tick).expect("the server records the view");
    assert_eq!(view.len(), held.len(), "entity count at tick {tick}");
    for &(id, source) in held {
        let source_frame = history.frame(source).expect("the source frame is in history");
        assert_eq!(view.get(id), source_frame.get(id), "entity {id} at tick {tick}");
    }
}

#[test]
fn round_trip_over_many_ticks_reproduces_every_view() {
    let schema = round_schema();
    let mut rng = SplitMix64(0x5EED_0001);
    let mut entities = Entities::new();
    let mut history = History::new(64);
    let mut cache = EncodeCache::new();
    let mut server = ClientState::new();
    let mut client = ClientViews::new();
    let mut buf = vec![0u8; ROOMY_BITS / 8];
    let mut totals = SnapshotStats::default();
    for tick in 0..600u32 {
        step(&mut rng, &mut entities, 200);
        fill(&schema, &mut history, tick, &entities);
        cache.reset(tick);
        let candidates = all_candidates(&entities);
        let (stats, bytes) =
            snapshot(&schema, &history, &mut cache, &mut server, tick, &candidates, ROOMY_BITS - 64, &mut buf);
        totals.full += stats.full;
        totals.delta += stats.delta;
        totals.removed += stats.removed;
        totals.skipped += stats.skipped;
        assert_eq!(stats.skipped, 0, "tick {tick}: a roomy budget must take everything");
        // 20% of the snapshots are lost on the way, so the baseline falls back over several ticks now and then.
        if rng.below(5) == 0 {
            continue;
        }
        let mut r = BitReader::new(&buf[..bytes]);
        assert_eq!(client.read_snapshot(&schema, &mut r), Ok(tick));
        check_view(&client, &server, &history, tick);
        server.on_delivered(tick);
    }
    assert!(totals.full > 0 && totals.delta > 0 && totals.removed > 0, "every kind of entry must occur: {totals:?}");
}

#[test]
fn unchanged_entities_cost_nothing_but_keep_their_view() {
    let schema = round_schema();
    let mut history = History::new(64);
    let mut cache = EncodeCache::new();
    let mut server = ClientState::new();
    let mut client = ClientViews::new();
    let mut buf = vec![0u8; 1024];
    let mut entities = Entities::new();
    entities.insert(3, (KindId(0), vec![100, 5]));
    entities.insert(9, (KindId(1), vec![7, 2]));
    for tick in 0..20u32 {
        fill(&schema, &mut history, tick, &entities);
        cache.reset(tick);
        let (stats, bytes) = snapshot(
            &schema,
            &history,
            &mut cache,
            &mut server,
            tick,
            &all_candidates(&entities),
            ROOMY_BITS - 64,
            &mut buf,
        );
        if tick > 0 {
            assert_eq!(stats.delta + stats.full, 0, "tick {tick}: nothing changed");
            assert_eq!(stats.bits, 48, "tick {tick}: the header alone (32 + distance 8 + count 8)");
        }
        let mut r = BitReader::new(&buf[..bytes]);
        assert_eq!(client.read_snapshot(&schema, &mut r), Ok(tick));
        check_view(&client, &server, &history, tick);
        server.on_delivered(tick);
    }
}

#[test]
fn source_frame_leaving_history_forces_a_full_record() {
    let schema = round_schema();
    let mut history = History::new(64);
    let mut cache = EncodeCache::new();
    let mut server = ClientState::new();
    let mut client = ClientViews::new();
    let mut buf = vec![0u8; 1024];
    let mut entities = Entities::new();
    entities.insert(1, (KindId(0), vec![10, 1]));
    entities.insert(2, (KindId(1), vec![20, 2]));
    let candidates = all_candidates(&entities);
    // Tick 0 sends both in full with a roomy budget. Afterwards only the header fits, so the changing entities stay
    // at their tick-0 source frame until that frame leaves the 64-slot history.
    for tick in 0..=70u32 {
        for (_, values) in entities.values_mut() {
            values[0] = (values[0] + 3) % 4096;
        }
        fill(&schema, &mut history, tick, &entities);
        cache.reset(tick);
        let budget = match tick {
            0 => ROOMY_BITS - 64,
            _ => 64,
        };
        let (stats, bytes) = snapshot(&schema, &history, &mut cache, &mut server, tick, &candidates, budget, &mut buf);
        match tick {
            0 => assert_eq!(stats.full, 2),
            1..=63 => {
                assert_eq!(stats.full, 0, "tick {tick}: nothing is mandatory yet");
                assert_eq!(stats.skipped, 2, "tick {tick}: both changed and left out");
            }
            64 => assert_eq!(stats.full, 2, "tick 64: the source frame of tick 0 has left the history"),
            _ => {}
        }
        let mut r = BitReader::new(&buf[..bytes]);
        assert_eq!(client.read_snapshot(&schema, &mut r), Ok(tick));
        check_view(&client, &server, &history, tick);
        server.on_delivered(tick);
    }
}

#[test]
fn reset_gives_a_full_snapshot_without_baseline() {
    let schema = round_schema();
    let mut rng = SplitMix64(0x5EED_0002);
    let mut entities = Entities::new();
    let mut history = History::new(64);
    let mut cache = EncodeCache::new();
    let mut server = ClientState::new();
    let mut client = ClientViews::new();
    let mut buf = vec![0u8; ROOMY_BITS / 8];
    for tick in 0..10u32 {
        step(&mut rng, &mut entities, 50);
        fill(&schema, &mut history, tick, &entities);
        cache.reset(tick);
        let (_, bytes) = snapshot(
            &schema,
            &history,
            &mut cache,
            &mut server,
            tick,
            &all_candidates(&entities),
            ROOMY_BITS - 64,
            &mut buf,
        );
        let mut r = BitReader::new(&buf[..bytes]);
        assert_eq!(client.read_snapshot(&schema, &mut r), Ok(tick));
        server.on_delivered(tick);
    }
    assert!(server.held(9).is_some());
    server.reset();
    assert!(server.held(9).is_none(), "reset forgets the sent views");
    step(&mut rng, &mut entities, 50);
    fill(&schema, &mut history, 10, &entities);
    cache.reset(10);
    let candidates = all_candidates(&entities);
    let (stats, bytes) =
        snapshot(&schema, &history, &mut cache, &mut server, 10, &candidates, ROOMY_BITS - 64, &mut buf);
    assert_eq!(stats.baseline, None);
    assert_eq!(stats.delta, 0);
    assert_eq!(stats.full as usize, candidates.len());
    let mut r = BitReader::new(&buf[..bytes]);
    assert_eq!(client.read_snapshot(&schema, &mut r), Ok(10));
    check_view(&client, &server, &history, 10);
}

#[test]
fn baseline_sixty_four_ticks_back_reads_through_the_shared_slot() {
    let schema = round_schema();
    let mut history = History::new(64);
    let mut cache = EncodeCache::new();
    let mut server = ClientState::new();
    let mut client = ClientViews::new();
    let mut buf = vec![0u8; 1024];
    let mut entities = Entities::new();
    entities.insert(4, (KindId(0), vec![1, 1]));
    entities.insert(5, (KindId(1), vec![2, 2]));
    let candidates = all_candidates(&entities);
    // Only tick 0 is acknowledged, so every snapshot up to tick 64 is written against tick 0, and tick 64 is the one
    // that lands in the slot the baseline occupies.
    for tick in 0..=65u32 {
        for (_, values) in entities.values_mut() {
            values[0] = (values[0] + 1) % 4096;
        }
        fill(&schema, &mut history, tick, &entities);
        cache.reset(tick);
        let (stats, bytes) =
            snapshot(&schema, &history, &mut cache, &mut server, tick, &candidates, ROOMY_BITS - 64, &mut buf);
        match tick {
            1..=64 => assert_eq!(stats.baseline, Some(0), "tick {tick}"),
            65 => assert_eq!(stats.baseline, None, "tick 65: tick 0 has left the view slots"),
            _ => {}
        }
        let mut r = BitReader::new(&buf[..bytes]);
        assert_eq!(client.read_snapshot(&schema, &mut r), Ok(tick));
        check_view(&client, &server, &history, tick);
        if tick == 0 {
            server.on_delivered(0);
        }
    }
}
