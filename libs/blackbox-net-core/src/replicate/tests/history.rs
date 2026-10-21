//! The history ring: slots are reused modulo the capacity and a stale tick is never returned.

use super::fixed_schema;
use crate::replicate::{History, KindId};

#[test]
fn ring_keeps_the_last_capacity_ticks() {
    let schema = fixed_schema();
    let mut history = History::new(4);
    assert_eq!(history.capacity(), 4);
    assert_eq!(history.latest_tick(), None);
    assert!(history.frame(0).is_none());
    for tick in 0..10u32 {
        let frame = history.begin(tick);
        assert_eq!(frame.tick(), tick);
        assert!(frame.is_empty());
        assert_eq!(frame.push(&schema, 1, KindId(0), &[tick, 0, 0]), Ok(()));
    }
    assert_eq!(history.latest_tick(), Some(9));
    for tick in 0..6 {
        assert!(history.frame(tick).is_none(), "tick {tick} should be overwritten");
    }
    for tick in 6..10 {
        let frame = history.frame(tick);
        assert_eq!(frame.and_then(|f| f.get(1)).map(|(_, values)| values[0]), Some(tick));
    }
    assert!(history.frame(10).is_none(), "a tick never begun is not held");
}

#[test]
fn zero_capacity_becomes_one_slot() {
    let schema = fixed_schema();
    let mut history = History::new(0);
    assert_eq!(history.capacity(), 1);
    assert_eq!(history.begin(5).push(&schema, 0, KindId(1), &[1]), Ok(()));
    assert!(history.frame(5).is_some());
    assert_eq!(history.begin(6).tick(), 6);
    assert!(history.frame(5).is_none());
    assert_eq!(history.latest_tick(), Some(6));
}

#[test]
fn begin_clears_the_slot_it_reuses() {
    let schema = fixed_schema();
    let mut history = History::new(2);
    assert_eq!(history.begin(0).push(&schema, 3, KindId(1), &[1]), Ok(()));
    let frame = history.begin(2);
    assert!(frame.is_empty());
    assert_eq!(frame.get(3), None);
}
