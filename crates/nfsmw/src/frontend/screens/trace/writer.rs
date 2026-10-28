//! Bounded, nonblocking telemetry transport. Only the worker touches the file after startup.

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const CAPACITY: usize = 256;

pub(super) struct Writer {
    sender: Option<mpsc::SyncSender<Value>>,
    worker: Option<JoinHandle<io::Result<()>>>,
    failed: Arc<AtomicBool>,
    started: Instant,
    sequence: u64,
    dropped: u64,
}

fn output_path(path: &Path, checkout: &Path) -> io::Result<PathBuf> {
    if !path.is_absolute() {
        return Err(io::Error::other("NFSMW_UI_TRACE must be an absolute path"));
    }
    let parent = path.parent().ok_or_else(|| io::Error::other("trace needs a parent directory"))?.canonicalize()?;
    if parent.starts_with(checkout.canonicalize()?) {
        return Err(io::Error::other("trace output must be outside the source checkout"));
    }
    Ok(parent.join(path.file_name().ok_or_else(|| io::Error::other("trace needs a file name"))?))
}

impl Writer {
    pub fn create(path: &Path, checkout: &Path) -> io::Result<Self> {
        let path = output_path(path, checkout)?;
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Self::for_file(file, CAPACITY)
    }

    fn for_file(file: File, capacity: usize) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let failed = Arc::new(AtomicBool::new(false));
        let worker_failed = failed.clone();
        let started = Instant::now();
        let unix_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos().to_string();
        let worker = std::thread::Builder::new().name("ui-trace".into()).spawn(move || {
            let result = write_stream(file, receiver);
            if let Err(error) = &result {
                worker_failed.store(true, Ordering::Relaxed);
                log::error!("UI trace writer stopped: {error}");
            }
            result
        })?;
        let mut writer = Self { sender: Some(sender), worker: Some(worker), failed, started, sequence: 0, dropped: 0 };
        writer.emit(
            0,
            "header",
            json!({"schema": "nfsmw-ui-trace-v1", "unix_ns": unix_ns,
            "pid": std::process::id(), "sample_period_ms": 33, "queue_capacity": capacity,
            "sound_evidence": "runtime_request_not_playback", "clock": "monotonic_since_header",
            "scope": "frontend_screen_host", "object_space": "local_before_presentation"}),
        );
        Ok(writer)
    }

    pub fn emit(&mut self, frame: u64, kind: &str, data: Value) {
        if self.failed.load(Ordering::Relaxed) {
            return;
        }
        let record = json!({"seq": self.sequence, "frame": frame,
            "elapsed_ns": self.started.elapsed().as_nanos() as u64,
            "kind": kind, "dropped_since_previous": self.dropped, "data": data});
        self.sequence += 1;
        let Some(sender) = &self.sender else { return };
        match sender.try_send(record) {
            Ok(()) => self.dropped = 0,
            Err(mpsc::TrySendError::Full(_)) => self.dropped += 1,
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.failed.store(true, Ordering::Relaxed);
                log::error!("UI trace writer disconnected");
            }
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        // Shutdown alone may wait for the writer. Gameplay never blocks on the disk.
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(json!({"seq": self.sequence, "kind": "end", "data": {
                "dropped_since_previous": self.dropped},
                "elapsed_ns": self.started.elapsed().as_nanos() as u64}));
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn write_stream(file: File, receiver: mpsc::Receiver<Value>) -> io::Result<()> {
    let mut file = BufWriter::with_capacity(64 * 1024, file);
    let mut last_flush = Instant::now();
    loop {
        match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(record) => {
                serde_json::to_writer(&mut file, &record)?;
                file.write_all(b"\n")?;
                if last_flush.elapsed() >= Duration::from_secs(1) {
                    file.flush()?;
                    last_flush = Instant::now();
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                file.flush()?;
                last_flush = Instant::now();
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return file.flush(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_queue_reports_loss_without_waiting() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut writer = Writer {
            sender: Some(sender),
            worker: None,
            failed: Arc::default(),
            started: Instant::now(),
            sequence: 0,
            dropped: 0,
        };
        writer.emit(1, "first", Value::Null);
        writer.emit(2, "lost", Value::Null);
        assert_eq!(writer.dropped, 1);
        assert_eq!(receiver.try_recv().unwrap()["seq"], 0);
        writer.emit(3, "next", Value::Null);
        let record = receiver.try_recv().unwrap();
        assert_eq!(record["seq"], 2);
        assert_eq!(record["dropped_since_previous"], 1);
        drop(receiver);
    }
}
