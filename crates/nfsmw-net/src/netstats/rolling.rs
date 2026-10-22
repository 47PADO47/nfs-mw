//! A sum over a rolling time window, in fixed buckets.

/// A count summed over the last `buckets × bucket_ms` milliseconds. Memory is the fixed bucket array; adding or reading
/// at an older time than the newest bucket counts into the newest.
#[derive(Debug, Clone)]
pub struct Rolling {
    bucket_ms: u64,
    buckets: Vec<u64>,
    /// The bucket index (time / bucket_ms) of the newest bucket.
    head: Option<u64>,
}

impl Rolling {
    /// A window of `buckets` buckets of `bucket_ms` each (both at least 1).
    pub fn new(bucket_ms: u64, buckets: usize) -> Self {
        Self { bucket_ms: bucket_ms.max(1), buckets: vec![0; buckets.max(1)], head: None }
    }

    /// The window length, milliseconds.
    pub fn window_ms(&self) -> u64 {
        self.bucket_ms * self.buckets.len() as u64
    }

    /// Count `n` at `now_ms`.
    pub fn add(&mut self, now_ms: u64, n: u64) {
        let index = self.roll(now_ms);
        let len = self.buckets.len() as u64;
        if let Some(slot) = self.buckets.get_mut((index % len) as usize) {
            *slot = slot.saturating_add(n);
        }
    }

    /// The sum over the window that ends at `now_ms`.
    pub fn sum(&mut self, now_ms: u64) -> u64 {
        self.roll(now_ms);
        self.buckets.iter().sum()
    }

    /// The sum per second over the window that ends at `now_ms`.
    pub fn per_second(&mut self, now_ms: u64) -> f64 {
        self.sum(now_ms) as f64 * 1000.0 / self.window_ms() as f64
    }

    /// Move the head to the bucket of `now_ms`, clearing the buckets passed over. Returns the head index.
    fn roll(&mut self, now_ms: u64) -> u64 {
        let index = now_ms / self.bucket_ms;
        let len = self.buckets.len() as u64;
        let Some(head) = self.head else {
            self.head = Some(index);
            return index;
        };
        if index <= head {
            return head;
        }
        let passed = (index - head).min(len);
        for step in 1..=passed {
            if let Some(slot) = self.buckets.get_mut(((head + step) % len) as usize) {
                *slot = 0;
            }
        }
        self.head = Some(index);
        index
    }
}
