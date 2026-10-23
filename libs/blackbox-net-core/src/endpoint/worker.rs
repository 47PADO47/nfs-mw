//! The thread behind every endpoint handle: a stop flag the loop checks each pass, and a join on drop so no thread
//! outlives its handle.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::thread::{self, JoinHandle};

/// A running net thread. Dropping it stops and joins the thread.
#[derive(Debug)]
pub(crate) struct Worker {
    stop: Arc<AtomicBool>,
    alive: Weak<()>,
    handle: Option<JoinHandle<()>>,
}

/// What the thread body gets: the stop flag to check, and a token it holds while it runs.
#[derive(Debug)]
pub(crate) struct Stop {
    flag: Arc<AtomicBool>,
    _alive: Arc<()>,
}

impl Stop {
    /// Whether the handle asked the thread to stop.
    pub(crate) fn requested(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

impl Worker {
    /// Starts a thread named `name` running `body`.
    pub(crate) fn spawn<F: FnOnce(Stop) + Send + 'static>(name: &str, body: F) -> io::Result<Self> {
        let flag = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(());
        let weak = Arc::downgrade(&alive);
        let stop = Stop { flag: Arc::clone(&flag), _alive: alive };
        let handle = thread::Builder::new().name(name.to_owned()).spawn(move || body(stop))?;
        Ok(Self { stop: flag, alive: weak, handle: Some(handle) })
    }

    /// Asks the thread to stop and waits for it. Idempotent.
    pub(crate) fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let Some(handle) = self.handle.take() else {
            return;
        };
        // A panic on the net thread has already ended it; there is nothing left to clean up.
        let _ = handle.join();
    }

    /// Whether the thread body has returned.
    pub(crate) fn is_finished(&self) -> bool {
        self.alive.strong_count() == 0
    }

    /// A token that dies with the thread body, for tests that check no thread is left behind.
    #[cfg(test)]
    pub(crate) fn probe(&self) -> Weak<()> {
        self.alive.clone()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}
