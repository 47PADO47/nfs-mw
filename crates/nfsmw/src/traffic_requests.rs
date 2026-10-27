//! Traffic cars asked for by name: a mod's `bb.traffic` `spawn` and the `traffic spawn` console command. The ask is a place on
//! the map and a car; the world's traffic manager serves it when the car's model is loaded (it asks for the model on the
//! first tick it sees the request), so a request can wait for a few seconds and is dropped after [`WAIT_SECONDS`].
//!
//! The queue is a plain list behind a lock, not a method of the `Scene` trait: the scenes that have no traffic never look
//! at it, and the one that has reads it once a tick.

use std::sync::{Mutex, PoisonError};

/// How long a request waits for its model and a place on the road, in seconds, before it is dropped.
pub const WAIT_SECONDS: f32 = 30.0;
/// The most requests waiting at once; further asks are refused.
pub const MAX_WAITING: usize = 32;

/// One ask: the car's folder (any case), the map position (x, y), the heading in radians from +X, and the seconds it may still
/// wait.
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnRequest {
    pub car: String,
    pub at: [f32; 2],
    pub heading: f32,
    pub seconds_left: f32,
}

static WAITING: Mutex<Vec<SpawnRequest>> = Mutex::new(Vec::new());

/// Ask for the traffic car `car` at `at`, facing `heading`. Returns whether the ask is queued (the queue is not full).
pub fn push(car: &str, at: [f32; 2], heading: f32) -> bool {
    let mut waiting = WAITING.lock().unwrap_or_else(PoisonError::into_inner);
    if waiting.len() >= MAX_WAITING {
        return false;
    }
    waiting.push(SpawnRequest { car: car.to_owned(), at, heading, seconds_left: WAIT_SECONDS });
    true
}

/// Take every waiting request, in the order they were asked. The caller puts back the ones it cannot serve yet.
pub fn take() -> Vec<SpawnRequest> {
    std::mem::take(&mut *WAITING.lock().unwrap_or_else(PoisonError::into_inner))
}

/// Put requests back at the front of the queue, keeping their order and their remaining wait.
pub fn put_back(mut requests: Vec<SpawnRequest>) {
    if requests.is_empty() {
        return;
    }
    let mut waiting = WAITING.lock().unwrap_or_else(PoisonError::into_inner);
    requests.append(&mut waiting);
    requests.truncate(MAX_WAITING);
    *waiting = requests;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The queue is shared by the whole test run: these checks run in one test so they cannot interleave.
    #[test]
    fn requests_wait_in_order_and_come_back_to_the_front() {
        let _ = take();
        assert!(push("HATCH", [10.0, 20.0], 0.5));
        assert!(push("VAN", [30.0, 40.0], 1.5));
        let mut asked = take();
        assert_eq!(asked.iter().map(|r| r.car.as_str()).collect::<Vec<_>>(), ["HATCH", "VAN"]);
        assert_eq!(asked[0].seconds_left, WAIT_SECONDS);
        assert!(take().is_empty(), "taking empties the queue");
        assert!(push("LATER", [0.0, 0.0], 0.0));
        let served = asked.remove(0);
        put_back(asked);
        let order: Vec<String> = take().into_iter().map(|r| r.car).collect();
        assert_eq!(order, ["VAN", "LATER"], "the unserved ask keeps its place before the newer one");
        assert_eq!(served.car, "HATCH");
        for n in 0..MAX_WAITING {
            assert!(push("X", [n as f32, 0.0], 0.0));
        }
        assert!(!push("X", [0.0, 0.0], 0.0), "a full queue refuses");
        let _ = take();
    }
}
