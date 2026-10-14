//! Optional, bounded controller evidence; never writes files in the audio callback.
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::Instant;

use blackbox_aems::{PlayerInputs, input};

#[derive(Clone, Default)]
pub(super) struct Trace(Arc<Inner>);

#[derive(Default)]
struct Inner {
    enabled: AtomicBool,
    next_instance: AtomicU64,
    logical_us: AtomicU64,
    session: Mutex<Option<Session>>,
}

struct Session {
    tx: mpsc::SyncSender<String>,
    worker: JoinHandle<()>,
    started: Instant,
    sequence: u64,
    dropped: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
}

#[derive(Default)]
pub(super) struct Record<'a> {
    pub event: &'a str,
    pub instance: u64,
    pub bank: &'a str,
    pub class: &'a str,
    pub logical_ms: f64,
    pub params: &'a [i32],
    pub player: Option<usize>,
    pub inputs: Option<&'a PlayerInputs>,
    pub success: Option<bool>,
    pub detail: &'a str,
}

fn quoted(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch < ' ' => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

impl Record<'_> {
    fn json(&self, sequence: u64, wall_ms: f64) -> String {
        let mut out = format!(
            "{{\"sequence\":{sequence},\"wall_ms\":{wall_ms},\"logical_ms\":{},\"instance\":{},\"event\":",
            self.logical_ms, self.instance
        );
        quoted(&mut out, self.event);
        out.push_str(",\"bank\":");
        quoted(&mut out, self.bank);
        out.push_str(",\"class\":");
        quoted(&mut out, self.class);
        let _ = write!(out, ",\"params\":{:?},\"player\":", self.params);
        match self.player {
            Some(p) => {
                let _ = write!(out, "{p}");
            }
            None => out.push_str("null"),
        }
        out.push_str(",\"inputs\":[");
        for k in 0..input::COUNT {
            if k > 0 {
                out.push(',');
            }
            match self.inputs.and_then(|v| v.get(k)) {
                Some(v) => {
                    let _ = write!(out, "{v}");
                }
                None => out.push_str("null"),
            }
        }
        out.push_str("],\"success\":");
        match self.success {
            Some(v) => {
                let _ = write!(out, "{v}");
            }
            None => out.push_str("null"),
        }
        out.push_str(",\"detail\":");
        quoted(&mut out, self.detail);
        out.push_str("}\n");
        out
    }
}

impl Trace {
    pub fn enabled(&self) -> bool {
        self.0.enabled.load(Ordering::Relaxed)
    }
    pub fn instance(&self) -> u64 {
        self.0.next_instance.fetch_add(1, Ordering::Relaxed) + 1
    }
    pub fn now_ms(&self) -> f64 {
        self.0.logical_us.load(Ordering::Relaxed) as f64 / 1000.0
    }
    pub fn advance(&self, dt: f32) {
        if dt.is_finite() && dt > 0.0 {
            self.0.logical_us.fetch_add((f64::from(dt) * 1e6).round() as u64, Ordering::Relaxed);
        }
    }
    pub fn start(&self, path: &Path) -> Result<(), String> {
        let file = File::create(path).map_err(|e| format!("audio trace: {e}"))?;
        self.stop();
        let (tx, rx) = mpsc::sync_channel::<String>(8192);
        let dropped = Arc::new(AtomicU64::new(0));
        let failed = Arc::new(AtomicBool::new(false));
        let (drop_count, write_failed) = (dropped.clone(), failed.clone());
        let worker = std::thread::spawn(move || {
            let mut writer = BufWriter::new(file);
            let mut count = 0;
            for line in rx {
                if writer.write_all(line.as_bytes()).is_err() {
                    write_failed.store(true, Ordering::Relaxed);
                    break;
                }
                count += 1;
                if count % 256 == 0 && writer.flush().is_err() {
                    write_failed.store(true, Ordering::Relaxed);
                    break;
                }
            }
            let end = format!(
                "{{\"event\":\"session_end\",\"records\":{count},\"dropped\":{},\"write_failed\":{}}}\n",
                drop_count.load(Ordering::Relaxed),
                write_failed.load(Ordering::Relaxed)
            );
            if writer.write_all(end.as_bytes()).and_then(|()| writer.flush()).is_err() {
                write_failed.store(true, Ordering::Relaxed);
            }
        });
        *self.0.session.lock().map_err(|e| e.to_string())? =
            Some(Session { tx, worker, started: Instant::now(), sequence: 0, dropped, failed });
        self.0.enabled.store(true, Ordering::Relaxed);
        self.note("session_start", "schema=1; class/voice words are requested values; backend pan/filter/pitch policy bounded; authored wet capability and effective row recorded separately");
        Ok(())
    }
    pub fn stop(&self) {
        self.0.enabled.store(false, Ordering::Relaxed);
        if let Ok(mut guard) = self.0.session.lock()
            && let Some(session) = guard.take()
        {
            drop(session.tx);
            let _ = session.worker.join();
        }
    }
    pub fn record(&self, record: Record<'_>) {
        if !self.enabled() {
            return;
        }
        let Ok(mut guard) = self.0.session.lock() else { return };
        let Some(session) = guard.as_mut() else { return };
        session.sequence += 1;
        if session.sequence > 1_000_000 || session.failed.load(Ordering::Relaxed) {
            session.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        let line = record.json(session.sequence, session.started.elapsed().as_secs_f64() * 1000.0);
        if session.tx.try_send(line).is_err() {
            session.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    pub fn note(&self, event: &str, detail: &str) {
        self.record(Record { event, detail, logical_ms: self.now_ms(), ..Default::default() });
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Ok(session) = self.session.get_mut()
            && let Some(session) = session.take()
        {
            drop(session.tx);
            let _ = session.worker.join();
        }
    }
}

#[cfg(test)]
mod tests;
