//! The one place the endpoints read the wall clock: a monotonic millisecond count from the thread's start. Everything
//! below the endpoint keeps taking `now_ms` from its caller.

use std::time::Instant;

/// Milliseconds since this clock was made, monotonic.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Clock {
    start: Instant,
}

impl Clock {
    /// A clock at 0 now.
    pub(crate) fn new() -> Self {
        Self { start: Instant::now() }
    }

    /// Milliseconds since [`Clock::new`].
    pub(crate) fn now_ms(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}
