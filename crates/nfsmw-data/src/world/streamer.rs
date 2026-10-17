//! Background loading of sections: a few worker threads read and parse
//! sections on request; the caller polls for results each frame.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

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

pub struct Streamer {
    requests: Option<Sender<Request>>,
    results: Receiver<StreamerEvent>,
    workers: Vec<std::thread::JoinHandle<()>>,
    /// Whether the renderer draws block-compressed textures; read by the workers for each section they load.
    bc_supported: Arc<AtomicBool>,
}

impl Streamer {
    /// Start `threads` workers reading from `stream_path`. Until [`Streamer::set_bc_supported`] says otherwise,
    /// the workers assume block-compressed textures can be drawn as they are.
    pub fn spawn(stream_path: PathBuf, sections: Vec<StreamingSection>, threads: usize) -> Self {
        let (req_tx, req_rx) = channel::<Request>();
        let (res_tx, res_rx) = channel();
        let req_rx = Arc::new(Mutex::new(req_rx));
        let sections = Arc::new(sections);
        let bc_supported = Arc::new(AtomicBool::new(true));
        let workers = (0..threads.max(1))
            .map(|n| {
                let (req_rx, res_tx, sections, path, bc) =
                    (req_rx.clone(), res_tx.clone(), sections.clone(), stream_path.clone(), bc_supported.clone());
                std::thread::Builder::new()
                    .name(format!("section-loader-{n}"))
                    .spawn(move || worker(&path, &sections, &req_rx, &res_tx, &bc))
                    .expect("spawning a section loader thread")
            })
            .collect();
        Self { requests: Some(req_tx), results: res_rx, workers, bc_supported }
    }

    /// Tell the workers whether the renderer draws block-compressed textures. Sections loaded after this decode the
    /// textures the renderer needs decoded (`SectionData::decode_for_upload`); the render thread checks the same rule.
    pub fn set_bc_supported(&self, supported: bool) {
        self.bc_supported.store(supported, Ordering::Relaxed);
    }

    pub fn request(&self, index: usize) {
        if let Some(tx) = &self.requests {
            let _ = tx.send(Request::Full(index));
        }
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
        self.requests = None; // closes the channel; workers exit after their current section
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
                data.decode_for_upload(bc_supported.load(Ordering::Relaxed));
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
