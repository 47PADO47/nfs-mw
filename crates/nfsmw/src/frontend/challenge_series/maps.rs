//! The map pictures of the events: `TRACK_MAP_<event id>` in `TrackMaps.bin` (63 MB, 1024 x 1024 each). The challenge series
//! screen reads the ones of the series on a worker thread when it opens, and the world map the one of the running race;
//! each adds them to the UI textures and drops them when it is left.

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use blackbox_tpk::{Texture, read_texture_packs_filtered};

use super::board;
use crate::ui::UiAssets;

const FILE: &str = "TRACKS/L2RA/TrackMaps.bin";

enum State {
    Idle,
    /// Behind a lock only so the screen's logic is `Sync`; the screen is the receiver's one user.
    Reading(Mutex<Receiver<Vec<Texture>>>),
    /// The pictures are in the assets under these name hashes.
    Loaded(Vec<u32>),
}

pub(crate) struct Maps(State);

impl Default for Maps {
    fn default() -> Self {
        Self(State::Idle)
    }
}

impl Maps {
    /// Starts reading the pictures with these name hashes. Does nothing when a read is under way or done.
    pub fn start(&mut self, wanted: Vec<u32>) {
        let Some(dir) = board::install().cloned() else { return };
        self.start_in(dir, wanted);
    }

    /// Like [`Maps::start`], reading from `dir`.
    pub fn start_in(&mut self, dir: game_install::GameDir, wanted: Vec<u32>) {
        if !matches!(self.0, State::Idle) {
            return;
        }
        let (send, receive) = channel();
        let spawned = std::thread::Builder::new().name("map-pictures".into()).spawn(move || {
            let _ = send.send(read(&dir, &wanted));
        });
        match spawned {
            Ok(_) => self.0 = State::Reading(Mutex::new(receive)),
            Err(e) => log::warn!("map pictures: no thread to read them: {e}"),
        }
    }

    /// Takes a finished read into the assets. Call every frame.
    pub fn poll(&mut self, assets: &UiAssets) {
        let State::Reading(receive) = &self.0 else { return };
        let textures = match receive.lock().unwrap_or_else(|e| e.into_inner()).try_recv() {
            Ok(textures) => textures,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Vec::new(),
        };
        let hashes: Vec<u32> = textures.iter().map(|t| t.name_hash).collect();
        log::info!("map pictures: {} loaded", hashes.len());
        assets.add_textures(textures);
        self.0 = State::Loaded(hashes);
    }

    /// Whether the pictures are in the assets.
    pub fn loaded(&self) -> bool {
        matches!(self.0, State::Loaded(_))
    }

    /// Drops the pictures from the assets (a read still under way is dropped with its receiver).
    pub fn release(&mut self, assets: &UiAssets) {
        if let State::Loaded(hashes) = &self.0 {
            assets.remove_textures(hashes);
        }
        self.0 = State::Idle;
    }
}

fn read(dir: &game_install::GameDir, wanted: &[u32]) -> Vec<Texture> {
    let data = match nfsmw_data::shared_unwrapped(dir, FILE) {
        Ok(data) => data,
        Err(e) => {
            log::warn!("map pictures: {FILE}: {e:#}");
            return Vec::new();
        }
    };
    match read_texture_packs_filtered(&data, |hash| wanted.contains(&hash)) {
        Ok(packs) => packs.into_iter().flat_map(|pack| pack.textures).collect(),
        Err(e) => {
            log::warn!("map pictures: {FILE}: {e:#}");
            Vec::new()
        }
    }
}
