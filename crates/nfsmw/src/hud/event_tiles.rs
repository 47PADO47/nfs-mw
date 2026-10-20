//! The minimap tiles of the running race's event (`TRACKS/L2RA/MINI_MAP_<event id>.BIN`), read on demand on a worker
//! thread when a race starts and dropped from the UI assets when it ends (`docs/specs/race-minimap.md` section 3).
//! 267 such files exist; none is read until a race asks for its own.

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use blackbox_minimap::TileSet;
use game_install::GameDir;

use crate::ui::UiAssets;

/// Reads the tiles of a map file by its stem (`MINI_MAP_10_2_1`).
pub type TileLoader = Arc<dyn Fn(&str) -> Result<TileSet> + Send + Sync>;

/// The loader of the install: `nfsmw_data::minimap::read_tiles`.
pub fn install_loader(dir: GameDir) -> TileLoader {
    Arc::new(move |stem| nfsmw_data::minimap::read_tiles(&dir, stem))
}

enum State {
    Idle,
    /// A worker is reading the event's file.
    Loading {
        event: String,
        /// Behind a lock only so the state is `Sync` (a Bevy resource); the HUD system is its one user.
        tiles: Mutex<Receiver<Result<TileSet>>>,
    },
    /// The event's tiles are in the UI assets under these name hashes.
    Ready {
        event: String,
        stem: String,
        hashes: Vec<u32>,
    },
}

/// Loads and releases the tiles of one event at a time.
pub struct EventTiles {
    loader: TileLoader,
    state: State,
    /// Events whose file could not be read (most pursuit challenges have none): not asked for again.
    failed: HashSet<String>,
}

impl EventTiles {
    pub fn new(loader: TileLoader) -> Self {
        Self { loader, state: State::Idle, failed: HashSet::new() }
    }

    /// Follows the wanted event (`None` when no event map is wanted). Call every frame. Tiles of another event are
    /// dropped from `assets`, a finished read is added to them, and a new event starts a read on a worker thread.
    pub fn update(&mut self, wanted: Option<&str>, assets: &UiAssets) {
        self.drop_unwanted(wanted, assets);
        self.finish_read(assets);
        let Some(event) = wanted else { return };
        if !matches!(self.state, State::Idle) || self.failed.contains(event) {
            return;
        }
        self.start_read(event);
    }

    /// The file stem of the wanted event's tiles once they are in the assets; `None` while they load or when the event
    /// has no file.
    pub fn ready_stem(&self) -> Option<&str> {
        match &self.state {
            State::Ready { stem, .. } => Some(stem),
            _ => None,
        }
    }

    /// The event whose tiles are loaded, if any.
    #[cfg(test)]
    pub fn ready_event(&self) -> Option<&str> {
        match &self.state {
            State::Ready { event, .. } => Some(event),
            _ => None,
        }
    }

    fn drop_unwanted(&mut self, wanted: Option<&str>, assets: &UiAssets) {
        let current = match &self.state {
            State::Idle => return,
            State::Loading { event, .. } | State::Ready { event, .. } => event.as_str(),
        };
        if wanted == Some(current) {
            return;
        }
        // A read still running finishes on its own; its result is dropped with the receiver.
        if let State::Ready { hashes, .. } = &self.state {
            assets.remove_textures(hashes);
        }
        self.state = State::Idle;
    }

    fn finish_read(&mut self, assets: &UiAssets) {
        let State::Loading { event, tiles } = &self.state else { return };
        let result = match tiles.lock().unwrap_or_else(|e| e.into_inner()).try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(anyhow::anyhow!("the tile reader stopped")),
        };
        let event = event.clone();
        match result {
            Ok(set) => {
                let hashes: Vec<u32> = set.tiles.iter().map(|t| t.name_hash).collect();
                let stem = nfsmw_data::minimap::event_stem(&event);
                assets.add_textures(set.tiles);
                log::info!("minimap: the tiles of event {event} are loaded");
                self.state = State::Ready { event, stem, hashes };
            }
            Err(e) => {
                log::info!("minimap: event {event} has no tiles ({e:#}); the stage map stays");
                self.failed.insert(event);
                self.state = State::Idle;
            }
        }
    }

    fn start_read(&mut self, event: &str) {
        let stem = nfsmw_data::minimap::event_stem(event);
        let loader = self.loader.clone();
        let (send, tiles) = channel();
        let spawned = std::thread::Builder::new().name("minimap-tiles".into()).spawn(move || {
            // The receiver is gone when the race ended before the read did.
            let _ = send.send(loader(&stem));
        });
        if let Err(e) = spawned {
            log::warn!("minimap: no thread for the tile reader: {e}");
            self.failed.insert(event.to_owned());
            return;
        }
        self.state = State::Loading { event: event.to_owned(), tiles: Mutex::new(tiles) };
    }
}

#[cfg(test)]
#[path = "event_tiles_tests.rs"]
mod tests;
