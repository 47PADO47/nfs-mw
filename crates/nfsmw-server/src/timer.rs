//! The fixed-step timer of the server loop (integration.md section 1): tick `n` is due at `start + n / 60 s`, computed
//! from the start every time, so rounding never accumulates into drift. The loop sleeps until shortly before the
//! deadline and spins the last [`SPIN_MARGIN`], because a sleep on Windows can overshoot by a millisecond or more.

use std::thread;
use std::time::{Duration, Instant};

/// The part of each wait that spins instead of sleeping.
pub const SPIN_MARGIN: Duration = Duration::from_micros(1500);
/// A loop further behind than this many ticks skips ahead instead of running every missed tick at once.
pub const MAX_CATCH_UP_TICKS: u64 = 6;

/// When each tick is due.
#[derive(Debug, Clone)]
pub struct TickTimer {
    start: Instant,
    hz: u64,
    next: u64,
    skipped: u64,
}

impl TickTimer {
    /// A timer whose tick 0 is due now, at `hz` ticks per second (at least 1).
    pub fn new(hz: u32) -> Self {
        Self { start: Instant::now(), hz: u64::from(hz.max(1)), next: 0, skipped: 0 }
    }

    /// The offset of tick `n` from the start: `n * 1 s / hz`, exact to the nanosecond.
    pub fn offset(&self, n: u64) -> Duration {
        let nanos = u128::from(n) * 1_000_000_000 / u128::from(self.hz);
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }

    /// Milliseconds since the start: the loop's `now_ms`.
    pub fn now_ms(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Ticks skipped because the loop fell too far behind.
    pub fn skipped(&self) -> u64 {
        self.skipped
    }

    /// Waits until the next tick is due and returns its number. When the loop is more than [`MAX_CATCH_UP_TICKS`]
    /// behind, the missed ticks are counted in [`TickTimer::skipped`] and the next tick is the one due now.
    pub fn wait(&mut self) -> u64 {
        let now = self.start.elapsed();
        let due_now = u128::from(self.hz) * now.as_nanos() / 1_000_000_000;
        let due_now = u64::try_from(due_now).unwrap_or(u64::MAX);
        if due_now > self.next + MAX_CATCH_UP_TICKS {
            self.skipped += due_now - self.next;
            self.next = due_now;
        }
        let deadline = self.start + self.offset(self.next);
        sleep_until(deadline);
        let tick = self.next;
        self.next += 1;
        tick
    }
}

/// Sleeps until `SPIN_MARGIN` before `deadline`, then spins until it.
fn sleep_until(deadline: Instant) {
    let now = Instant::now();
    if deadline <= now {
        return;
    }
    let left = deadline - now;
    if left > SPIN_MARGIN {
        thread::sleep(left - SPIN_MARGIN);
    }
    while Instant::now() < deadline {
        std::hint::spin_loop();
        thread::yield_now();
    }
}

#[cfg(test)]
mod tests {
    use super::TickTimer;
    use std::time::Duration;

    #[test]
    fn deadlines_are_exact_multiples_of_the_period() {
        let timer = TickTimer::new(60);
        assert_eq!(timer.offset(60), Duration::from_secs(1));
        assert_eq!(timer.offset(3600 * 60), Duration::from_secs(3600));
        assert_eq!(timer.offset(1), Duration::from_nanos(16_666_666));
        assert_eq!(timer.offset(2), Duration::from_nanos(33_333_333));
    }

    #[test]
    fn ticks_come_in_order_and_on_time() {
        let mut timer = TickTimer::new(60);
        let ticks: Vec<u64> = (0..6).map(|_| timer.wait()).collect();
        assert_eq!(ticks, vec![0, 1, 2, 3, 4, 5]);
        assert!(timer.now_ms() >= 83, "five periods passed");
    }
}
