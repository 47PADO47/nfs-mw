//! The priority accumulator under a budget too small for every change: every entity is sent, and none goes stale for long.

use super::snapshot::{check_view, snapshot};
use crate::bits::BitReader;
use crate::replicate::{ClientState, ClientViews, EncodeCache, FieldDesc, Frame, History, KindId, Schema};

/// Entities in the staleness scene.
const ENTITIES: u16 = 20;
/// Ticks run before the staleness is measured, so the views have settled.
const WARM_UP: u32 = 30;
/// Ticks measured.
const TICKS: u32 = 300;

/// A schema with one kind of 16-bit diff fields, for the staleness scene.
fn walk_schema() -> Schema {
    let mut schema = Schema::new();
    let added = schema.add_kind(&[FieldDesc { bits: 16, diff: true }]);
    assert_eq!(added, Ok(KindId(0)));
    schema
}

/// Runs the staleness scene with the given weight per entity and returns the largest staleness (ticks since the source
/// of an entry the client holds) seen after the warm-up, and the number of skipped changes in total.
fn largest_staleness(weight: fn(u16) -> f32) -> (u32, u32) {
    let schema = walk_schema();
    let mut history = History::new(64);
    let mut cache = EncodeCache::new();
    let mut server = ClientState::new();
    let mut client = ClientViews::new();
    let mut values: Vec<u32> = (0..ENTITIES).map(|id| u32::from(id) * 1000).collect();
    let candidates: Vec<(u16, f32)> = (0..ENTITIES).map(|id| (id, weight(id))).collect();
    // A budget of the header plus about four small deltas: 16 changes compete for it every tick.
    let budget = 64 + 4 * 40;
    let mut buf = vec![0u8; 1024];
    let mut worst = 0u32;
    let mut skipped_total = 0u32;
    for tick in 0..TICKS {
        for value in values.iter_mut() {
            *value = (*value + 400 + (tick * 37 + *value) % 600) % 65_536;
        }
        let frame = history.begin(tick);
        for (id, value) in values.iter().enumerate() {
            let pushed = frame.push(&schema, id as u16, KindId(0), &[*value]);
            assert!(pushed.is_ok());
        }
        cache.reset(tick);
        let (stats, bytes) = snapshot(&schema, &history, &mut cache, &mut server, tick, &candidates, budget, &mut buf);
        skipped_total += stats.skipped;
        let mut r = BitReader::new(&buf[..bytes]);
        assert_eq!(client.read_snapshot(&schema, &mut r), Ok(tick));
        check_view(&client, &server, &history, tick);
        server.on_delivered(tick);
        if tick < WARM_UP {
            continue;
        }
        let held = server.held(tick).unwrap_or(&[]);
        for &(_, source) in held {
            worst = worst.max(tick - source);
        }
    }
    (worst, skipped_total)
}

#[test]
fn equal_weights_send_every_entity_within_a_bounded_staleness() {
    let (worst, skipped) = largest_staleness(|_| 1.0);
    assert!(skipped > 0, "the budget must bind for the test to mean anything");
    assert!(worst <= 12, "largest staleness {worst} ticks");
}

#[test]
fn uneven_weights_still_bound_the_staleness_of_light_entities() {
    let (worst, skipped) = largest_staleness(|id| 1.0 + f32::from(id % 2));
    assert!(skipped > 0, "the budget must bind for the test to mean anything");
    assert!(worst <= 16, "largest staleness {worst} ticks");
}

#[test]
fn still_entities_cost_nothing_beside_a_moving_one() {
    let schema = walk_schema();
    let mut history = History::new(64);
    let mut cache = EncodeCache::new();
    let mut server = ClientState::new();
    let mut client = ClientViews::new();
    let mut buf = vec![0u8; 1024];
    let candidates: Vec<(u16, f32)> = (0..3u16).map(|id| (id, 1.0)).collect();
    for tick in 0..60u32 {
        // Entity 0 changes every tick; entities 1 and 2 are still.
        let frame: &mut Frame = history.begin(tick);
        assert!(frame.push(&schema, 0, KindId(0), &[tick * 50]).is_ok());
        assert!(frame.push(&schema, 1, KindId(0), &[7]).is_ok());
        assert!(frame.push(&schema, 2, KindId(0), &[9]).is_ok());
        cache.reset(tick);
        let (stats, bytes) =
            snapshot(&schema, &history, &mut cache, &mut server, tick, &candidates, 1024 * 8, &mut buf);
        if tick > 0 {
            assert_eq!(stats.skipped, 0, "tick {tick}: only the moving entity is written");
            assert_eq!(stats.delta, 1, "tick {tick}");
        }
        let mut r = BitReader::new(&buf[..bytes]);
        assert_eq!(client.read_snapshot(&schema, &mut r), Ok(tick));
        check_view(&client, &server, &history, tick);
        server.on_delivered(tick);
    }
}
