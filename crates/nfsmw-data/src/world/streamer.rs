//! Background loading of sections: a few worker threads read and parse
//! sections on request; the caller polls for results each frame.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};

use blackbox_streaming::StreamingSection;

use super::{SectionData, load_section};

pub enum StreamerEvent {
    Loaded(Box<SectionData>),
    Failed { index: usize, error: String },
    Collision { index: usize, result: Result<Vec<blackbox_collision::CollisionPack>, String> },
}

enum Request {
    Full(usize),
    Collision(usize),
}

/// Work a worker does on a section it has parsed, beyond reading it: whatever the caller can compute without a
/// renderer (it stores the result in [`SectionData::prepared`]). Installed once ([`Streamer::set_prepare`]).
pub type Prepare = dyn Fn(&mut SectionData) + Send + Sync;

/// What the caller tells the workers while they run: stop, drop these requests, prepare sections like this.
struct Control {
    stop: AtomicBool,
    /// Per section index: the caller no longer wants the request that is still queued. [`Streamer::request`] clears it.
    cancelled: Vec<AtomicBool>,
    prepare: OnceLock<Box<Prepare>>,
}

impl Control {
    fn new(sections: usize) -> Self {
        Self {
            stop: AtomicBool::new(false),
            cancelled: (0..sections).map(|_| AtomicBool::new(false)).collect(),
            prepare: OnceLock::new(),
        }
    }

    fn set_cancelled(&self, index: usize, cancelled: bool) {
        let Some(flag) = self.cancelled.get(index) else { return };
        flag.store(cancelled, Ordering::Relaxed);
    }

    /// Whether a worker that took `request` off the queue should drop it unread.
    fn skips(&self, request: &Request) -> bool {
        match request {
            Request::Full(index) => self.cancelled.get(*index).is_some_and(|flag| flag.load(Ordering::Relaxed)),
            Request::Collision(_) => false,
        }
    }
}

pub struct Streamer {
    requests: Option<Sender<Request>>,
    results: Receiver<StreamerEvent>,
    workers: Vec<std::thread::JoinHandle<()>>,
    /// Whether the renderer draws block-compressed textures; read by the workers for each section they load.
    bc_supported: Arc<AtomicBool>,
    control: Arc<Control>,
}

impl Streamer {
    /// Start `threads` workers reading from `stream_path`. Until [`Streamer::set_bc_supported`] says otherwise,
    /// the workers assume block-compressed textures can be drawn as they are.
    pub fn spawn(stream_path: PathBuf, sections: Vec<StreamingSection>, threads: usize) -> Self {
        let (req_tx, req_rx) = channel::<Request>();
        let (res_tx, res_rx) = channel();
        let req_rx = Arc::new(Mutex::new(req_rx));
        let control = Arc::new(Control::new(sections.len()));
        let sections = Arc::new(sections);
        let bc_supported = Arc::new(AtomicBool::new(true));
        let workers = (0..threads.max(1))
            .map(|n| {
                let (req_rx, res_tx, sections, path, bc, control) = (
                    req_rx.clone(),
                    res_tx.clone(),
                    sections.clone(),
                    stream_path.clone(),
                    bc_supported.clone(),
                    control.clone(),
                );
                std::thread::Builder::new()
                    .name(format!("section-loader-{n}"))
                    .spawn(move || worker(&path, &sections, &req_rx, &res_tx, &bc, &control))
                    .expect("spawning a section loader thread")
            })
            .collect();
        Self { requests: Some(req_tx), results: res_rx, workers, bc_supported, control }
    }

    /// Tell the workers whether the renderer draws block-compressed textures. Sections loaded after this decode the
    /// textures the renderer needs decoded (`SectionData::decode_for_upload`); the render thread checks the same rule.
    pub fn set_bc_supported(&self, supported: bool) {
        self.bc_supported.store(supported, Ordering::Relaxed);
    }

    /// Have the workers run `prepare` on every section they load from now on, after the textures are decoded. Only the
    /// first call counts. Sections loaded before it arrive unprepared, so the receiver must cope with both.
    pub fn set_prepare(&self, prepare: Box<Prepare>) {
        let _ = self.control.prepare.set(prepare);
    }

    pub fn request(&self, index: usize) {
        let Some(tx) = &self.requests else { return };
        self.control.set_cancelled(index, false);
        let _ = tx.send(Request::Full(index));
    }

    /// Drop the queued request for section `index` if no worker has started it. A section already being read still
    /// arrives, and the receiver ignores it. A later [`Streamer::request`] for the same index is not affected.
    pub fn cancel(&self, index: usize) {
        self.control.set_cancelled(index, true);
    }

    pub fn request_collision(&self, index: usize) {
        if let Some(tx) = &self.requests {
            let _ = tx.send(Request::Collision(index));
        }
    }

    /// A finished section, if any.
    pub fn poll(&self) -> Option<StreamerEvent> {
        self.results.try_recv().ok()
    }
}

impl Drop for Streamer {
    fn drop(&mut self) {
        // Workers leave after the section they are on: the queued requests are not worked off.
        self.control.stop.store(true, Ordering::Relaxed);
        self.requests = None;
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }
}

fn worker(
    path: &std::path::Path,
    sections: &[StreamingSection],
    requests: &Mutex<Receiver<Request>>,
    results: &Sender<StreamerEvent>,
    bc_supported: &AtomicBool,
    control: &Control,
) {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            log::error!("cannot open {}: {e}", path.display());
            return;
        }
    };
    loop {
        let next = requests.lock().map(|rx| rx.recv());
        let Ok(Ok(request)) = next else { return };
        if control.stop.load(Ordering::Relaxed) {
            return;
        }
        if control.skips(&request) {
            continue;
        }
        // Idle channel waiting is excluded; scopes measure each requested read/parse/decode job.
        let _trace = blackbox_trace::scope("Streaming worker: section job");
        let index = match request {
            Request::Collision(index) => {
                let result = sections
                    .get(index)
                    .ok_or_else(|| "no such collision section".to_owned())
                    .and_then(|s| super::section::load_collision(&mut file, s).map_err(|e| format!("{e:#}")));
                if results.send(StreamerEvent::Collision { index, result }).is_err() {
                    return;
                }
                continue;
            }
            Request::Full(index) => index,
        };
        let event = match sections.get(index).map(|s| load_section(&mut file, index, s)) {
            Some(Ok(mut data)) => {
                let _decode = blackbox_trace::scope("Streaming worker: texture decode");
                data.decode_for_upload(bc_supported.load(Ordering::Relaxed));
                if let Some(prepare) = control.prepare.get() {
                    let _prepare = blackbox_trace::scope("Streaming worker: placement prep");
                    prepare(&mut data);
                }
                StreamerEvent::Loaded(Box::new(data))
            }
            Some(Err(e)) => StreamerEvent::Failed { index, error: format!("{e:#}") },
            None => StreamerEvent::Failed { index, error: "no such section".into() },
        };
        if results.send(event).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancelled_request_is_skipped_until_it_is_requested_again() {
        let control = Control::new(3);
        assert!(!control.skips(&Request::Full(1)));
        control.set_cancelled(1, true);
        assert!(control.skips(&Request::Full(1)));
        assert!(!control.skips(&Request::Full(0)), "only the cancelled section");
        assert!(!control.skips(&Request::Collision(1)), "collision requests are never cancelled");
        control.set_cancelled(1, false);
        assert!(!control.skips(&Request::Full(1)));
    }

    #[test]
    fn cancelling_a_section_the_stream_does_not_have_is_harmless() {
        let control = Control::new(1);
        control.set_cancelled(9, true);
        assert!(!control.skips(&Request::Full(9)));
    }

    #[test]
    fn dropping_a_streamer_does_not_work_off_its_queue() {
        // A stream file that cannot be opened ends the workers at once; the drop must still return.
        let streamer = Streamer::spawn(PathBuf::from("this-file-does-not-exist.bun"), Vec::new(), 2);
        for index in 0..1000 {
            streamer.request(index);
        }
        drop(streamer);
    }
}
