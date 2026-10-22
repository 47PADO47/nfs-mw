use std::thread;

use super::{latest, ring};

#[test]
fn a_ring_keeps_order_and_drops_the_oldest_when_full() {
    let (mut tx, mut rx) = ring::<u32>(3);
    for value in 0..3 {
        assert_eq!(tx.push(value), None);
    }
    assert_eq!(tx.push(3), Some(0), "the oldest is displaced");
    assert_eq!(tx.push(4), Some(1));
    assert_eq!(tx.dropped(), 2);
    assert_eq!(rx.dropped(), 2);
    assert_eq!(rx.len(), 3);
    assert_eq!(rx.capacity(), 3);
    let drained: Vec<u32> = std::iter::from_fn(|| rx.pop()).collect();
    assert_eq!(drained, vec![2, 3, 4]);
    assert!(rx.is_empty());
}

#[test]
fn a_ring_of_capacity_zero_holds_one() {
    let (mut tx, mut rx) = ring::<u8>(0);
    tx.push(1);
    tx.push(2);
    assert_eq!(rx.pop(), Some(2));
    assert_eq!(rx.pop(), None);
}

#[test]
fn a_ring_across_threads_loses_only_what_it_counts() {
    let (mut tx, mut rx) = ring::<u64>(64);
    let producer = thread::spawn(move || {
        for value in 0..100_000u64 {
            tx.push(value);
        }
        tx.dropped()
    });
    let mut received = Vec::new();
    while !producer.is_finished() {
        while let Some(value) = rx.pop() {
            received.push(value);
        }
        thread::yield_now();
    }
    let dropped = producer.join().expect("the producer finishes");
    while let Some(value) = rx.pop() {
        received.push(value);
    }
    assert!(received.windows(2).all(|pair| pair[0] < pair[1]), "order is kept");
    assert_eq!(received.len() as u64 + dropped, 100_000, "every value is received or counted as dropped");
    assert_eq!(received.last(), Some(&99_999), "the newest value always arrives");
}

#[test]
fn a_ring_half_sees_the_other_go() {
    let (tx, rx) = ring::<u8>(4);
    assert!(!rx.is_disconnected());
    assert!(!tx.is_disconnected());
    drop(tx);
    assert!(rx.is_disconnected());
}

#[test]
fn the_latest_cell_gives_the_newest_value() {
    let (mut writer, mut reader) = latest(&0u32);
    assert_eq!(*reader.read(), 0);
    writer.publish(1);
    writer.publish(2);
    assert!(reader.has_update());
    assert_eq!(*reader.read(), 2, "the reader skips to the newest");
    assert!(!reader.has_update());
    assert!(writer.consumed());
    assert_eq!(writer.published(), 2);
}

#[test]
fn the_latest_cell_fills_in_place() {
    let (mut writer, mut reader) = latest(&vec![0u8; 4]);
    writer.buffer_mut().copy_from_slice(&[1, 2, 3, 4]);
    writer.publish_in_place();
    assert_eq!(reader.read().as_slice(), &[1, 2, 3, 4]);
}

#[test]
fn the_latest_cell_across_threads_never_goes_backwards() {
    let (mut writer, mut reader) = latest(&0u64);
    let producer = thread::spawn(move || {
        for value in 1..=50_000u64 {
            writer.publish(value);
        }
    });
    let mut last = 0;
    while last < 50_000 {
        let value = *reader.read();
        assert!(value >= last, "values only move forward");
        last = value;
        thread::yield_now();
    }
    producer.join().expect("the producer finishes");
}
