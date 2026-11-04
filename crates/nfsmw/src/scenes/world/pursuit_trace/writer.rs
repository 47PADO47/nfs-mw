//! The pursuit trace's writer thread: records go through a bounded channel and are serialised and written there, so
//! the game never waits on the disk. A full channel drops the record (counted); an IO error stops writing (reported);
//! the session stops writing at its size budget. Nothing here panics the game.
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

use super::records::{Event, Sample};
use super::session::Session;

/// Records waiting for the thread: about 12 s of samples.
const QUEUE: usize = 256;
/// The session's size budget, bytes (the original recorder's is the same).
pub(super) const MAX_BYTES: u64 = 512 * 1024 * 1024;

pub(super) enum Record {
    Sample(Box<Sample>),
    Event(Box<Event>),
    /// A new `session.json` (written at once, and again with the counters at the end).
    Session(Box<Session>),
}

/// What the thread has done, read by the F5 panel.
#[derive(Default)]
pub(super) struct Counters {
    pub samples: AtomicU64,
    pub events: AtomicU64,
    pub bytes: AtomicU64,
    /// Records the full channel refused.
    pub dropped: AtomicU64,
    /// Records not written because the size budget was reached.
    pub over_budget: AtomicU64,
    pub error: Mutex<Option<String>>,
}

impl Counters {
    pub fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|e| e.clone())
    }

    fn fail(&self, message: String) {
        log::warn!("pursuit trace: {message}");
        if let Ok(mut error) = self.error.lock() {
            error.get_or_insert(message);
        }
    }
}

pub(super) struct Writer {
    tx: mpsc::SyncSender<Record>,
    worker: Option<JoinHandle<()>>,
    pub counters: Arc<Counters>,
}

impl Writer {
    /// Starts the thread writing into `dir`, which it creates.
    pub fn start(dir: PathBuf) -> std::io::Result<Writer> {
        let (tx, rx) = mpsc::sync_channel(QUEUE);
        let counters = Arc::new(Counters::default());
        let shared = counters.clone();
        let worker = std::thread::Builder::new().name("pursuit-trace".into()).spawn(move || run(&dir, &rx, &shared))?;
        Ok(Writer { tx, worker: Some(worker), counters })
    }

    /// Hands a record to the thread without waiting; a full queue drops it.
    pub fn send(&self, record: Record) {
        if self.tx.try_send(record).is_err() {
            self.counters.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// The thread has written everything and stopped.
    pub fn finished(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

struct Files {
    samples: BufWriter<File>,
    events: BufWriter<File>,
}

fn open(dir: &Path) -> std::io::Result<Files> {
    std::fs::create_dir_all(dir)?;
    Ok(Files {
        samples: BufWriter::with_capacity(1 << 20, File::create(dir.join("samples.jsonl"))?),
        events: BufWriter::with_capacity(1 << 16, File::create(dir.join("events.jsonl"))?),
    })
}

fn run(dir: &Path, rx: &mpsc::Receiver<Record>, counters: &Counters) {
    let mut files = match open(dir) {
        Ok(files) => Some(files),
        Err(e) => {
            counters.fail(format!("cannot create {}: {e}", dir.display()));
            None
        }
    };
    let mut session: Option<Box<Session>> = None;
    let mut line = Vec::with_capacity(64 * 1024);
    // The sender is dropped when the recording stops: the loop ends after the last record.
    while let Ok(record) = rx.recv() {
        line.clear();
        let (counter, written) = match &record {
            Record::Session(meta) => {
                session = Some(meta.clone());
                write_session(dir, meta, counters);
                continue;
            }
            Record::Sample(sample) => (&counters.samples, serde_json::to_writer(&mut line, sample)),
            Record::Event(event) => (&counters.events, serde_json::to_writer(&mut line, event)),
        };
        if let Err(e) = written {
            counters.fail(format!("a record could not be serialised: {e}"));
            continue;
        }
        line.push(b'\n');
        if counters.bytes.load(Ordering::Relaxed) + line.len() as u64 > MAX_BYTES {
            counters.over_budget.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        let Some(out) = files.as_mut() else { continue };
        let target = match record {
            Record::Sample(_) => &mut out.samples,
            _ => &mut out.events,
        };
        if let Err(e) = target.write_all(&line) {
            counters.fail(format!("writing the trace failed: {e}"));
            files = None;
            continue;
        }
        counters.bytes.fetch_add(line.len() as u64, Ordering::Relaxed);
        counter.fetch_add(1, Ordering::Relaxed);
    }
    if let Some(mut out) = files
        && let Err(e) = out.samples.flush().and_then(|_| out.events.flush())
    {
        counters.fail(format!("flushing the trace failed: {e}"));
    }
    if let Some(mut meta) = session {
        meta.finish(counters);
        write_session(dir, &meta, counters);
    }
}

/// Writes `session.json` through a temporary file, so a reader never sees half of it.
fn write_session(dir: &Path, meta: &Session, counters: &Counters) {
    let partial = dir.join("session.json.partial");
    let result = serde_json::to_vec_pretty(meta)
        .map_err(std::io::Error::other)
        .and_then(|bytes| std::fs::write(&partial, bytes))
        .and_then(|_| std::fs::rename(&partial, dir.join("session.json")));
    if let Err(e) = result {
        counters.fail(format!("writing session.json failed: {e}"));
    }
}
