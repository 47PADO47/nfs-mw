use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crossbeam_queue::ArrayQueue;

/// What the two halves of a ring share.
#[derive(Debug)]
struct Shared<T> {
    queue: ArrayQueue<T>,
    dropped: AtomicU64,
}

/// Makes a bounded ring of `capacity` entries (at least 1) and returns its two halves. The sender never blocks: when
/// the ring is full, the oldest entry is dropped and counted.
pub fn ring<T>(capacity: usize) -> (RingSender<T>, RingReceiver<T>) {
    let shared = Arc::new(Shared { queue: ArrayQueue::new(capacity.max(1)), dropped: AtomicU64::new(0) });
    (RingSender { shared: Arc::clone(&shared) }, RingReceiver { shared })
}

/// The producing half of a [`ring`]. One thread pushes.
#[derive(Debug)]
pub struct RingSender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> RingSender<T> {
    /// Pushes `value`. If the ring is full, the oldest entry is dropped, counted and returned.
    pub fn push(&mut self, value: T) -> Option<T> {
        let displaced = self.shared.queue.force_push(value);
        if displaced.is_some() {
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
        }
        displaced
    }

    /// Entries dropped so far because the ring was full.
    pub fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }

    /// Whether the receiving half is gone.
    pub fn is_disconnected(&self) -> bool {
        Arc::strong_count(&self.shared) < 2
    }
}

/// The consuming half of a [`ring`]. One thread pops.
#[derive(Debug)]
pub struct RingReceiver<T> {
    shared: Arc<Shared<T>>,
}

impl<T> RingReceiver<T> {
    /// Takes the oldest entry, or `None` when the ring is empty. Never blocks.
    pub fn pop(&mut self) -> Option<T> {
        self.shared.queue.pop()
    }

    /// Entries waiting.
    pub fn len(&self) -> usize {
        self.shared.queue.len()
    }

    /// Whether nothing is waiting.
    pub fn is_empty(&self) -> bool {
        self.shared.queue.is_empty()
    }

    /// The most entries the ring holds.
    pub fn capacity(&self) -> usize {
        self.shared.queue.capacity()
    }

    /// Entries dropped so far because the ring was full.
    pub fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }

    /// Whether the sending half is gone.
    pub fn is_disconnected(&self) -> bool {
        Arc::strong_count(&self.shared) < 2
    }
}
