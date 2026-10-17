//! The one-shot sounds the mods ask for (`bb.audio` `play`): one sound, by its bank and its index in that bank, played once on
//! the sound-effects group. The mod side queues them with [`request`]; the audio plugin plays them each frame with
//! [`service`]. The port has no lookup from an event name to a sound yet (the `.csi` tables are not read), so a sound is
//! named by its bank and index.

use std::cell::{Cell, RefCell};

use kira::sound::static_sound::StaticSoundHandle;

use super::volume::decibels;
use super::{Audio, Group};
use crate::scenes::sounds::BANK_FOLDERS;

/// A sound a mod asked for, not yet played.
#[derive(Clone, Debug, PartialEq)]
pub struct OneShot {
    /// The bank: a path under `SOUND/` (`FE/FE_MB.abk`), or a file name looked up in the sound folders (`FE_MB.abk`).
    pub bank: String,
    pub index: usize,
    /// The volume as an amplitude (1.0 is the sound as it is).
    pub volume: f32,
    /// The id the mod was given.
    pub id: u64,
}

thread_local! {
    /// The one-shots queued since the audio plugin last took them. Mods run on the main thread, as the audio plugin does.
    static QUEUE: RefCell<Vec<OneShot>> = const { RefCell::new(Vec::new()) };
    static LAST_ID: Cell<u64> = const { Cell::new(0) };
}

/// Queues a one-shot for the audio plugin and returns its id (1 and up, in the order they were asked for).
pub fn request(bank: String, index: usize, volume: f32) -> u64 {
    let id = LAST_ID.with(|last| {
        let id = last.get() + 1;
        last.set(id);
        id
    });
    QUEUE.with(|queue| queue.borrow_mut().push(OneShot { bank, index, volume, id }));
    id
}

/// Takes the one-shots queued since the last call, in the order they were asked for.
fn take() -> Vec<OneShot> {
    QUEUE.with(|queue| std::mem::take(&mut *queue.borrow_mut()))
}

/// The path of the bank `name` names, when `exists` says the bank is in the install. A name with a folder is checked as it
/// is; a bare file name is looked up in each sound folder, in the order of [`BANK_FOLDERS`].
pub fn bank_path(name: &str, mut exists: impl FnMut(&str) -> bool) -> Option<String> {
    if name.contains('/') {
        return exists(name).then(|| name.to_owned());
    }
    for folder in BANK_FOLDERS {
        let path = format!("{folder}/{name}");
        if exists(&path) {
            return Some(path);
        }
    }
    None
}

/// The one-shots that are playing, kept so their handles stay alive until the sound ends.
#[derive(Default)]
pub struct OneShotPlayer {
    playing: Vec<StaticSoundHandle>,
}

/// Plays the one-shots queued since the last call, on the sound-effects group. A sound that cannot play is logged: the mod
/// is not waiting for it, so there is no one to return the error to.
pub fn service(audio: &mut Audio, player: &mut OneShotPlayer) {
    for shot in take() {
        player.play(audio, &shot);
    }
    player.playing.retain(|handle| !matches!(handle.state(), kira::sound::PlaybackState::Stopped));
}

impl OneShotPlayer {
    fn play(&mut self, audio: &mut Audio, shot: &OneShot) {
        let Some(bank) = bank_path(&shot.bank, |path| audio.bank_bytes(path).is_ok()) else {
            log::warn!("bb.audio play {}: no sound bank of that name in the install", shot.bank);
            return;
        };
        let data = match audio.bank_sound(&bank, shot.index) {
            Ok(data) => data.volume(decibels(shot.volume)),
            Err(e) => {
                log::warn!("bb.audio play {bank} #{}: {e}", shot.index);
                return;
            }
        };
        match audio.play(Group::Sfx, data) {
            Ok(handle) => self.playing.push(handle),
            Err(e) => log::warn!("bb.audio play {bank} #{}: {e}", shot.index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bank_with_a_folder_is_checked_as_it_is() {
        let installed = ["FE/FE_MB.abk"];
        let exists = |path: &str| installed.contains(&path);
        assert_eq!(bank_path("FE/FE_MB.abk", exists), Some("FE/FE_MB.abk".to_owned()));
        assert_eq!(bank_path("GLOBAL/FE_MB.abk", exists), None);
    }

    #[test]
    fn a_bare_bank_name_is_found_in_its_folder() {
        let installed = ["SHIFTING/GEAR.abk", "GLOBAL/GEAR.abk"];
        let exists = |path: &str| installed.contains(&path);
        assert_eq!(bank_path("GEAR.abk", exists), Some("GLOBAL/GEAR.abk".to_owned()), "GLOBAL comes before SHIFTING");
        assert_eq!(bank_path("NOSUCH.abk", |_| false), None);
    }

    #[test]
    fn requests_are_numbered_and_taken_once_in_order() {
        let first = request("FE/FE_MB.abk".to_owned(), 2, 0.5);
        let second = request("GLOBAL/GEAR.abk".to_owned(), 0, 1.0);
        assert_eq!(second, first + 1);
        let shots = take();
        assert_eq!(shots.iter().map(|s| (s.id, s.index)).collect::<Vec<_>>(), [(first, 2), (second, 0)]);
        assert!(take().is_empty(), "a request is taken once");
    }
}
