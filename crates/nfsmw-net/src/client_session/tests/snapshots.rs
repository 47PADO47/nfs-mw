use super::*;
use crate::frames::PursuitStatus;
use crate::message::BaselineLost;

const OTHER: u16 = 7;

/// Sends a snapshot every 3 ticks for `ms`, the remote car moving along x, each a delta against the one before
/// (the first full). Returns the last tick sent.
fn stream(h: &mut Harness, ms: u64) -> Tick {
    let mut last: Option<(Tick, EntityView, EntityView)> = None;
    let until = h.now + ms;
    while h.now < until {
        let tick = server_tick(h.now) as Tick;
        if last.as_ref().is_some_and(|(prev, ..)| tick < prev + 3) {
            h.frame();
            h.answer_pings();
            continue;
        }
        let own = car_view(ENTITY, Some(SLOT), 0.0);
        let other = car_view(OTHER, Some(1), tick as f32 * 0.1);
        let body = match &last {
            None => body(&h.schema, tick, 0, &[Entry::Full(&own), Entry::Full(&other)]),
            Some((prev, _, old)) => body(&h.schema, tick, tick - prev, &[Entry::Delta { old, new: &other }]),
        };
        h.snapshot(tick, tick, Some(&own_block(0.0)), &body);
        last = Some((tick, own, other));
        h.frame();
        h.answer_pings();
    }
    last.map_or(0, |(tick, ..)| tick)
}

#[test]
fn snapshots_feed_the_own_car_and_the_remote_entities() {
    let mut h = Harness::synced();
    stream(&mut h, 1000);
    let counters = h.session.counters();
    assert!(counters.snapshots > 15, "{counters:?}");
    assert_eq!((counters.missing_baselines, counters.bad_snapshots, counters.bad_frames), (0, 0, 0));
    assert!(h.session.own_pose().is_some(), "the first own block places the car");
    let mut display = Vec::new();
    h.session.remote_entities(&mut display);
    let ids: Vec<u16> = display.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![OTHER], "the own car is not a remote entity");
    let (_, global) = h.session.global().expect("a global block");
    assert_eq!(global.pursuit, PursuitStatus::None);
    assert!(h.session.net_stats().rtt_ms().is_some());
}

#[test]
fn a_removed_entity_leaves_at_display_time() {
    let mut h = Harness::synced();
    let last = stream(&mut h, 500);
    let tick = last + 3;
    h.snapshot(tick, tick, None, &body(&h.schema, tick, 3, &[Entry::Remove(OTHER)]));
    h.frame();
    assert_eq!(h.session.counters().missing_baselines, 0);
    for _ in 0..40 {
        h.frame();
        h.answer_pings();
    }
    let mut display = Vec::new();
    h.session.remote_entities(&mut display);
    assert!(display.is_empty(), "{display:?}");
}

fn part(h: &mut Harness, part: u8, parts: u8, body: &[u8]) {
    let header = SnapshotHeader { part, parts, ack_input: 0 };
    let own = own_block(0.0);
    let own = (part == 0).then_some(&own);
    let mut out = Vec::new();
    write_snapshot(&h.schema, header, own, None, body, &mut out).unwrap();
    h.unreliable(FRAME_SNAPSHOT, out);
}

#[test]
fn parts_are_merged_in_any_order() {
    let mut h = Harness::synced();
    let tick = server_tick(h.now) as Tick;
    let (a, b) = (car_view(3, Some(0), 1.0), car_view(9, Some(4), 2.0));
    let first = body(&h.schema, tick, 0, &[Entry::Full(&a)]);
    let second = body(&h.schema, tick, 0, &[Entry::Full(&b)]);
    part(&mut h, 1, 2, &second);
    h.frame();
    assert_eq!(h.session.counters().snapshots, 0, "half a snapshot is not decoded");
    part(&mut h, 0, 2, &first);
    h.frame();
    assert_eq!(h.session.counters().snapshots, 1);
    // A delta naming the merged view as its baseline decodes: both parts' entities are in it.
    let moved = car_view(9, Some(4), 3.0);
    let delta = body(
        &h.schema,
        tick + 3,
        3,
        &[Entry::Delta { old: &a, new: &car_view(3, Some(0), 1.5) }, Entry::Delta { old: &b, new: &moved }],
    );
    h.snapshot(tick + 3, tick + 3, None, &delta);
    h.frame();
    assert_eq!(h.session.counters().snapshots, 2);
    assert_eq!(h.session.counters().missing_baselines, 0);
}

#[test]
fn a_lost_baseline_is_reported_once_per_half_second() {
    let mut h = Harness::synced();
    h.sent();
    let tick = server_tick(h.now) as Tick;
    let old = car_view(OTHER, Some(1), 0.0);
    let new = car_view(OTHER, Some(1), 1.0);
    for offset in 0..3 {
        let delta = body(&h.schema, tick + offset, 5, &[Entry::Delta { old: &old, new: &new }]);
        h.snapshot(tick + offset, tick, None, &delta);
        h.frame();
    }
    assert_eq!(h.session.counters().missing_baselines, 3);
    let lost: Vec<_> = h
        .answer_pings()
        .into_iter()
        .filter_map(|sent| match sent {
            Sent::Reliable { bytes, .. } => Some(decode(&bytes).unwrap()),
            Sent::Unreliable { .. } => None,
        })
        .collect();
    assert_eq!(lost, vec![Message::from(BaselineLost { tick })]);
}

#[test]
fn an_older_snapshot_still_decodes_and_garbage_bodies_are_counted() {
    let mut h = Harness::synced();
    let tick = server_tick(h.now) as Tick;
    let other = car_view(OTHER, Some(1), 0.0);
    h.snapshot(tick, tick, None, &body(&h.schema, tick, 0, &[Entry::Full(&other)]));
    h.snapshot(tick - 6, tick, None, &body(&h.schema, tick - 6, 0, &[Entry::Full(&other)]));
    h.frame();
    assert_eq!(h.session.counters().snapshots, 2);
    let mut garbage = body(&h.schema, tick + 3, 0, &[]);
    garbage.extend_from_slice(&[0xFF, 0xFF, 0xFF]);
    h.snapshot(tick + 3, tick, None, &garbage);
    h.unreliable(FRAME_SNAPSHOT, vec![0x10, 0, 0, 0, 1]);
    h.unreliable(0x55, vec![1, 2, 3]);
    h.frame();
    let counters = h.session.counters();
    assert_eq!((counters.bad_snapshots, counters.bad_frames), (1, 2));
}

#[test]
fn snapshots_before_the_seat_are_ignored() {
    let mut h = Harness::new(config());
    h.connect();
    let body = body(&h.schema, 600, 0, &[]);
    h.snapshot(600, 600, None, &body);
    h.frame();
    assert_eq!(h.session.counters().snapshots, 0);
}
