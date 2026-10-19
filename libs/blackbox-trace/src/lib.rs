//! On-demand instrumentation, independent of logging, renderers and application frameworks.
//! Disabled scopes only read an atomic. Recording holds bounded data in memory; export does I/O.
mod export;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Hard upper bound, including captures started by callers outside the UI.
pub const MAX_EVENTS: usize = 1_000_000;
static GLOBAL: OnceLock<Recorder> = OnceLock::new();
static NEXT_THREAD: AtomicU64 = AtomicU64::new(1);
thread_local! { static THREAD: u64 = NEXT_THREAD.fetch_add(1, Ordering::Relaxed); }

/// The application-wide recorder. Libraries can add scopes without owning the capture lifecycle.
pub fn global() -> &'static Recorder {
    GLOBAL.get_or_init(Recorder::default)
}

/// A wall-time scope on the current CPU thread. Does not measure CPU active time or GPU execution.
pub fn scope(name: &'static str) -> Scope<'static> {
    global().scope(name)
}

#[derive(Default)]
pub struct Recorder {
    active: AtomicU64,
    next: AtomicU64,
    capture: Mutex<Option<Capture>>,
}

pub struct Capture {
    epoch: Instant,
    generation: u64,
    limit: usize,
    events: Vec<Event>,
    threads: BTreeMap<u64, String>,
    dropped: u64,
}

struct Event {
    name: &'static str,
    thread: u64,
    us: f64,
    kind: Kind,
}

enum Kind {
    Scope(f64),
    Counter(Vec<(&'static str, f64)>),
    Note(String),
}

/// A scope is tied to its capture generation; old guards cannot write into a new capture.
pub struct Scope<'a> {
    recorder: &'a Recorder,
    token: Option<(u64, Instant, &'static str, u64)>,
    // Guards must end on their starting thread so exported thread nesting remains meaningful.
    _thread: std::marker::PhantomData<*const ()>,
}

impl Recorder {
    /// Start a new capture. Returns false if one is already active. No files are opened.
    pub fn start(&self, limit: usize) -> bool {
        let mut state = self.capture.lock().unwrap_or_else(|e| e.into_inner());
        if state.is_some() {
            return false;
        }
        let generation = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        let limit = limit.clamp(1, MAX_EVENTS);
        *state = Some(Capture {
            epoch: Instant::now(),
            generation,
            limit,
            events: Vec::with_capacity(limit),
            threads: BTreeMap::new(),
            dropped: 0,
        });
        self.active.store(generation, Ordering::Release);
        true
    }

    pub fn recording(&self) -> bool {
        self.active.load(Ordering::Relaxed) != 0
    }

    pub fn session(&self) -> u64 {
        self.active.load(Ordering::Relaxed)
    }

    pub fn scope(&self, name: &'static str) -> Scope<'_> {
        let generation = self.active.load(Ordering::Relaxed);
        let token = (generation != 0).then(|| (generation, Instant::now(), name, THREAD.with(|id| *id)));
        Scope { recorder: self, token, _thread: std::marker::PhantomData }
    }

    /// Numeric values at their observation time. Invalid values are omitted, never replaced with zero.
    pub fn counter(&self, name: &'static str, values: &[(&'static str, f64)]) {
        if !self.recording() {
            return;
        }
        let values: Vec<_> = values.iter().copied().filter(|(_, value)| value.is_finite()).take(32).collect();
        if values.is_empty() {
            return;
        }
        self.push(name, Kind::Counter(values));
    }

    /// Bounded text, suitable for bookmarks, settings changes and capture context.
    pub fn note(&self, name: &'static str, text: &str) {
        if !self.recording() {
            return;
        }
        self.push(name, Kind::Note(text.chars().take(4096).collect()));
    }

    fn push(&self, name: &'static str, kind: Kind) {
        let generation = self.active.load(Ordering::Relaxed);
        if generation == 0 {
            return;
        }
        let mut state = self.capture.lock().unwrap_or_else(|e| e.into_inner());
        let Some(capture) = state.as_mut().filter(|c| c.generation == generation) else { return };
        let thread = THREAD.with(|id| *id);
        capture.thread(thread);
        capture.push(Event { name, thread, us: capture.epoch.elapsed().as_secs_f64() * 1e6, kind });
    }

    /// Detach all recorded data. The caller can export it on a worker thread.
    pub fn stop(&self) -> Option<Capture> {
        let mut state = self.capture.lock().unwrap_or_else(|e| e.into_inner());
        self.active.store(0, Ordering::Release);
        state.take()
    }
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        let Some((generation, start, name, thread)) = self.token else { return };
        if self.recorder.active.load(Ordering::Relaxed) != generation {
            return;
        }
        let end = Instant::now();
        let mut state = self.recorder.capture.lock().unwrap_or_else(|e| e.into_inner());
        let Some(capture) = state.as_mut().filter(|c| c.generation == generation) else { return };
        let start = start.max(capture.epoch);
        capture.thread(thread);
        capture.push(Event {
            name,
            thread,
            us: start.duration_since(capture.epoch).as_secs_f64() * 1e6,
            kind: Kind::Scope(end.saturating_duration_since(start).as_secs_f64() * 1e6),
        });
    }
}

impl Capture {
    fn push(&mut self, event: Event) {
        if self.events.len() == self.limit {
            self.dropped += 1;
            return;
        }
        self.events.push(event);
    }

    fn thread(&mut self, id: u64) {
        // Thread metadata is bounded independently of the event buffer.
        if self.threads.len() >= 256 {
            return;
        }
        self.threads
            .entry(id)
            .or_insert_with(|| std::thread::current().name().unwrap_or("CPU thread").chars().take(128).collect());
    }

    pub fn events(&self) -> usize {
        self.events.len()
    }
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
    pub fn elapsed(&self) -> Duration {
        self.epoch.elapsed()
    }
}
