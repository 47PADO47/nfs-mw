//! Background loading of sections: a few worker threads read and parse
//! sections on request; the caller polls for results each frame.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use blackbox_streaming::StreamingSection;

use super::{SectionData, load_section};

pub enum StreamerEvent {
    Loaded(Box<SectionData>),
    Failed { index: usize, error: String },
}

pub struct Streamer {
    requests: Option<Sender<usize>>,
    results: Receiver<StreamerEvent>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

impl Streamer {
    /// Start `threads` workers reading from `stream_path`.
    pub fn spawn(stream_path: PathBuf, sections: Vec<StreamingSection>, threads: usize) -> Self {
        let (req_tx, req_rx) = channel::<usize>();
        let (res_tx, res_rx) = channel();
        let req_rx = Arc::new(Mutex::new(req_rx));
        let sections = Arc::new(sections);
        let workers = (0..threads.max(1))
            .map(|n| {
                let (req_rx, res_tx, sections, path) =
                    (req_rx.clone(), res_tx.clone(), sections.clone(), stream_path.clone());
                std::thread::Builder::new()
                    .name(format!("section-loader-{n}"))
                    .spawn(move || worker(&path, &sections, &req_rx, &res_tx))
                    .expect("spawning a section loader thread")
            })
            .collect();
        Self { requests: Some(req_tx), results: res_rx, workers }
    }

    pub fn request(&self, index: usize) {
        if let Some(tx) = &self.requests {
            let _ = tx.send(index);
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
    requests: &Mutex<Receiver<usize>>,
    results: &Sender<StreamerEvent>,
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
        let Ok(Ok(index)) = next else { return };
        let event = match sections.get(index).map(|s| load_section(&mut file, index, s)) {
            Some(Ok(data)) => StreamerEvent::Loaded(Box::new(data)),
            Some(Err(e)) => StreamerEvent::Failed { index, error: format!("{e:#}") },
            None => StreamerEvent::Failed { index, error: "no such section".into() },
        };
        if results.send(event).is_err() {
            return;
        }
    }
}
