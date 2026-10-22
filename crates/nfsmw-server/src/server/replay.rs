//! The server's replay (MP-71): the whole world at the player snapshot rate, every broadcast event, and each round's
//! result record, written to a folder. The stream is the replay (sync-model.md section 6): the snapshot bodies are
//! the replicate snapshots of a viewer that sees every entity and acknowledges everything at once, with a key
//! snapshot (no baseline) every second so a player can seek.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use blackbox_net_core::bits::BitWriter;
use blackbox_net_core::replicate::ClientState;
use nfsmw_net::record::{DEFAULT_MAX_PENDING, RecordKind, Recorder, ReplayHeader};
use nfsmw_net::results::RaceRecord;
use nfsmw_net::schema::GameSchema;

use super::ServerSetup;
use super::view::WorldView;

/// A key snapshot every this many ticks.
const KEY_TICKS: u32 = 60;
/// The recorder is drained to the file every this many ticks.
const FLUSH_TICKS: u32 = 60;
/// Room for one whole-world snapshot.
const SNAPSHOT_BYTES: usize = 64 * 1024;

/// The open replay file and its recorder.
pub(crate) struct ReplayWriter {
    dir: PathBuf,
    stem: String,
    file: File,
    recorder: Recorder,
    viewer: ClientState,
    candidates: Vec<(u16, f32)>,
    buf: Vec<u8>,
    last_key: Option<u32>,
    last_flush: u32,
    finished: bool,
}

impl ReplayWriter {
    /// Creates `<dir>/session-<unix seconds>.nfsrpl` for a world starting at `start_tick`.
    pub(crate) fn create(dir: &Path, setup: &ServerSetup, start_tick: u32) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
        let stem = format!("session-{seconds}");
        let file = File::create(dir.join(format!("{stem}.nfsrpl")))?;
        let build = &setup.build;
        let header =
            ReplayHeader::new(RecordKind::Server, setup.schema.schema_id(), build.sim_build, build.data_id, start_tick);
        log::info!("recording to {}", dir.join(format!("{stem}.nfsrpl")).display());
        Ok(Self {
            dir: dir.to_path_buf(),
            stem,
            file,
            recorder: Recorder::new(&header, DEFAULT_MAX_PENDING),
            viewer: ClientState::new(),
            candidates: Vec::new(),
            buf: vec![0; SNAPSHOT_BYTES],
            last_key: None,
            last_flush: start_tick,
            finished: false,
        })
    }

    /// Records an encoded event of `tick`.
    pub(crate) fn event(&mut self, tick: u32, bytes: &[u8]) {
        if let Err(err) = self.recorder.event(tick, bytes) {
            log::debug!("replay: event dropped: {err:?}");
        }
    }

    /// Records the world of `tick` when it falls on the snapshot rate (`interval` ticks).
    pub(crate) fn snapshot(&mut self, view: &mut WorldView, schema: &GameSchema, tick: u32, interval: u32) {
        if self.finished || !tick.is_multiple_of(interval.max(1)) {
            return;
        }
        let key = self.last_key.is_none_or(|last| tick.saturating_sub(last) >= KEY_TICKS);
        if key {
            self.viewer.reset();
            self.last_key = Some(tick);
        }
        view.everything(&mut self.candidates);
        self.buf.fill(0);
        let mut w = BitWriter::new(&mut self.buf);
        let written = self.viewer.write_snapshot(
            schema.schema(),
            &view.history,
            &mut view.cache,
            tick,
            &self.candidates,
            SNAPSHOT_BYTES * 8,
            &mut w,
        );
        if written.is_err() || w.align().is_err() {
            self.viewer.reset();
            return;
        }
        let len = w.bytes_written();
        self.viewer.on_delivered(tick);
        if let Err(err) = self.recorder.snapshot(tick, &self.buf[..len], key) {
            log::debug!("replay: snapshot dropped: {err:?}");
            self.last_key = None;
        }
        if tick.saturating_sub(self.last_flush) >= FLUSH_TICKS {
            self.last_flush = tick;
            self.flush();
        }
    }

    /// Writes a round's result record next to the replay as `<stem>-round-<n>.nmrr`.
    pub(crate) fn save_record(&mut self, record: &RaceRecord, round: u32) {
        let mut bytes = Vec::new();
        record.encode(&mut bytes);
        let path = self.dir.join(format!("{}-round-{round}.nmrr", self.stem));
        if let Err(err) = fs::write(&path, bytes) {
            log::warn!("cannot write {}: {err}", path.display());
        }
    }

    /// Appends what the recorder holds to the file.
    fn flush(&mut self) {
        let bytes = self.recorder.take();
        if let Err(err) = self.file.write_all(&bytes) {
            log::warn!("replay: write failed: {err}");
        }
    }

    /// Writes the rest, the index and the trailer.
    pub(crate) fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.flush();
        match self.recorder.finish() {
            Ok(bytes) => {
                if let Err(err) = self.file.write_all(&bytes).and_then(|()| self.file.flush()) {
                    log::warn!("replay: write failed: {err}");
                }
            }
            Err(err) => log::warn!("replay: cannot finish: {err:?}"),
        }
    }
}
