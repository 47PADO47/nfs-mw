//! Bank sounds decoded off the frame thread.
//!
//! Decoding a bank sound (EA-XA to PCM to `kira` frames) takes milliseconds, and a car's first seconds ask for dozens of
//! them. [`Decoder`] keeps every decoded sound in one table shared with a named worker thread: a bank is handed to the
//! worker (`preload`) as soon as it is known, and a later [`Decoder::acquire`] finds the sound ready. Each sound is decoded
//! once. A sound is claimed (`Decoding`) by whoever starts first; the other side, if it needs the sound meanwhile,
//! waits for the claimant instead of decoding it again. A sound nobody has claimed yet is decoded by the caller at
//! once, as before, so a frame never waits behind the worker's queue.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use ea_audio::abk::{Bank, SoundKind};
use game_install::GameDir;
use kira::sound::static_sound::StaticSoundData;

use super::pcm;

type Key = (String, usize);

/// A decoded sound and the channel count of its recording.
#[derive(Clone)]
pub(super) struct Decoded {
    pub data: StaticSoundData,
    pub channels: u16,
}

#[allow(clippy::large_enum_variant)]
enum Slot {
    Decoding,
    Ready(Decoded),
    Failed(String),
}

#[derive(Default)]
struct Shared {
    slots: Mutex<HashMap<Key, Slot>>,
    changed: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, HashMap<Key, Slot>> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Claims the sound if nobody has: `None` when it is decoding, decoded or failed already.
fn claim_free(shared: &Arc<Shared>, key: Key) -> Option<Claim> {
    let mut slots = shared.lock();
    if slots.contains_key(&key) {
        return None;
    }
    slots.insert(key.clone(), Slot::Decoding);
    Some(Claim { shared: shared.clone(), key, done: false })
}

/// The right to decode one sound. Dropped without a result (a panic), it frees the sound for another try.
pub(super) struct Claim {
    shared: Arc<Shared>,
    key: Key,
    done: bool,
}

impl Claim {
    /// Stores the result. A failure stays for whoever asks next, who takes it away (so the next ask tries again).
    fn finish(mut self, result: Result<Decoded, String>) {
        let slot = match result {
            Ok(decoded) => Slot::Ready(decoded),
            Err(error) => Slot::Failed(error),
        };
        self.shared.lock().insert(self.key.clone(), slot);
        self.shared.changed.notify_all();
        self.done = true;
    }

    /// Decodes the claimed sound of `bank` from its bytes (or the error that reading them gave) and stores it. A failure
    /// is returned and not kept, as the sound was asked for by the caller.
    pub fn decode(self, bank: &str, bytes: Result<Arc<Vec<u8>>, String>) -> Result<Decoded, String> {
        let index = self.key.1;
        let result = bytes.and_then(|bytes| {
            let parsed = Bank::parse(&bytes).map_err(|e| format!("{bank}: {e}"))?;
            decode(&parsed, bank, index)
        });
        // On a failure the claim is dropped, which frees the sound; anyone waiting for it tries for itself.
        let decoded = result?;
        self.finish(Ok(decoded.clone()));
        Ok(decoded)
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        self.shared.lock().remove(&self.key);
        self.shared.changed.notify_all();
    }
}

/// What a request for a sound finds.
#[allow(clippy::large_enum_variant)]
pub(super) enum Found {
    Ready(Decoded),
    /// The worker failed to decode it.
    Failed(String),
    /// Nobody has it: the caller decodes it through the claim.
    Mine(Claim),
}

/// Which sounds of a bank the worker decodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Preload {
    /// Every sound of the bank (the small banks of one car).
    All,
    /// The sounds the bank's sample tables name, which are the ones its modules can play.
    Tables,
}

struct Request {
    bank: String,
    which: Preload,
}

/// The decoded sounds of the session and the worker that fills them ahead of use.
#[derive(Default)]
pub(super) struct Decoder {
    shared: Arc<Shared>,
    worker: Option<Sender<Request>>,
    /// The banks handed to the worker, so each goes once.
    requested: HashSet<String>,
}

impl Decoder {
    /// Hands `bank` (a path under `SOUND/`) to the worker to decode ahead of use. A bank goes once; without a worker
    /// thread (it failed to start) nothing is decoded ahead and the sounds are decoded when asked for.
    pub fn preload(&mut self, dir: &GameDir, bank: &str, which: Preload) {
        if self.requested.contains(bank) {
            return;
        }
        if !self.start_worker(dir) {
            return;
        }
        let Some(worker) = &self.worker else { return };
        self.requested.insert(bank.to_owned());
        if worker.send(Request { bank: bank.to_owned(), which }).is_err() {
            log::warn!("sound decoding: the worker is gone, {bank} decodes when played");
        }
    }

    /// Starts the worker thread if it is not running. False when it cannot start.
    fn start_worker(&mut self, dir: &GameDir) -> bool {
        if self.worker.is_some() {
            return true;
        }
        let (sender, receiver) = channel();
        let shared = self.shared.clone();
        let dir = dir.clone();
        let spawned =
            std::thread::Builder::new().name("audio-decode".into()).spawn(move || run(&shared, &dir, receiver));
        if let Err(error) = spawned {
            log::warn!("sound decoding: no worker thread: {error}");
            return false;
        }
        self.worker = Some(sender);
        true
    }

    /// Sound `index` of `bank`: decoded already, or after the worker when it is decoding it, else the caller's to decode.
    pub fn acquire(&self, bank: &str, index: usize) -> Found {
        let key = (bank.to_owned(), index);
        let mut slots = self.shared.lock();
        loop {
            match slots.get(&key) {
                Some(Slot::Ready(decoded)) => return Found::Ready(decoded.clone()),
                Some(Slot::Failed(_)) => {
                    if let Some(Slot::Failed(error)) = slots.remove(&key) {
                        return Found::Failed(error);
                    }
                }
                Some(Slot::Decoding) => {
                    slots = self.shared.changed.wait(slots).unwrap_or_else(PoisonError::into_inner);
                }
                None => break,
            }
        }
        slots.insert(key.clone(), Slot::Decoding);
        Found::Mine(Claim { shared: self.shared.clone(), key, done: false })
    }
}

fn decode(bank: &Bank<'_>, name: &str, index: usize) -> Result<Decoded, String> {
    let decoded = bank.decode(index).map_err(|e| format!("{name} #{index}: {e}"))?;
    Ok(Decoded { data: pcm::sound(&decoded), channels: decoded.channels })
}

/// The worker: reads each requested bank once and decodes the sounds nobody has claimed.
fn run(shared: &Arc<Shared>, dir: &GameDir, requests: Receiver<Request>) {
    for request in requests {
        preload_bank(shared, dir, &request);
    }
}

fn preload_bank(shared: &Arc<Shared>, dir: &GameDir, request: &Request) {
    let bank = &request.bank;
    let bytes = match dir.read(&format!("SOUND/{bank}")) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::debug!("sound decoding: {bank}: {error}");
            return;
        }
    };
    let parsed = match Bank::parse(&bytes) {
        Ok(parsed) => parsed,
        Err(error) => {
            log::debug!("sound decoding: {bank}: {error}");
            return;
        }
    };
    for index in indices(&parsed, request.which) {
        let Some(claim) = claim_free(shared, (bank.clone(), index)) else { continue };
        claim.finish(decode(&parsed, bank, index));
    }
}

/// The sound numbers `which` selects, in bank order and without repeats.
fn indices(bank: &Bank<'_>, which: Preload) -> Vec<usize> {
    let all = bank.sounds().iter().map(|sound| sound.index);
    let named: Vec<usize> = match which {
        Preload::All => all.collect(),
        Preload::Tables => bank
            .sample_tables()
            .iter()
            .flat_map(|table| &table.entries)
            .filter(|entry| entry.kind == SoundKind::Ram && !entry.is_dummy())
            .map(|entry| entry.index as usize)
            .collect(),
    };
    let mut seen = HashSet::new();
    named.into_iter().filter(|index| seen.insert(*index)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoded() -> Decoded {
        let pcm = ea_audio::Pcm { sample_rate: 1000, channels: 1, samples: vec![0; 4], loop_range: None };
        Decoded { data: pcm::sound(&pcm), channels: 1 }
    }

    fn key(index: usize) -> Key {
        ("A.abk".into(), index)
    }

    #[test]
    fn a_claimed_sound_is_not_claimed_again() {
        let shared = Arc::new(Shared::default());
        let claim = claim_free(&shared, key(1)).expect("free");
        assert!(claim_free(&shared, key(1)).is_none());
        claim.finish(Ok(decoded()));
        assert!(claim_free(&shared, key(1)).is_none(), "a decoded sound stays decoded");
        assert!(claim_free(&shared, key(2)).is_some());
    }

    #[test]
    fn a_dropped_claim_frees_the_sound() {
        let shared = Arc::new(Shared::default());
        drop(claim_free(&shared, key(1)).expect("free"));
        assert!(claim_free(&shared, key(1)).is_some());
    }

    #[test]
    fn a_ready_sound_is_served_and_an_unknown_one_is_claimed() {
        let decoder = Decoder::default();
        claim_free(&decoder.shared, key(1)).expect("free").finish(Ok(decoded()));
        assert!(matches!(decoder.acquire("A.abk", 1), Found::Ready(sound) if sound.channels == 1));
        assert!(matches!(decoder.acquire("A.abk", 2), Found::Mine(_)));
    }

    #[test]
    fn a_failure_is_taken_once_and_then_tried_again() {
        let decoder = Decoder::default();
        claim_free(&decoder.shared, key(1)).expect("free").finish(Err("bad".into()));
        assert!(matches!(decoder.acquire("A.abk", 1), Found::Failed(error) if error == "bad"));
        assert!(matches!(decoder.acquire("A.abk", 1), Found::Mine(_)));
    }

    #[test]
    fn an_unreadable_bank_fails_the_claim_and_frees_the_sound() {
        let decoder = Decoder::default();
        let Found::Mine(claim) = decoder.acquire("A.abk", 1) else { panic!("free") };
        assert_eq!(claim.decode("A.abk", Err("no bank".into())).err().as_deref(), Some("no bank"));
        assert!(decoder.shared.lock().is_empty());
    }

    #[test]
    fn a_sound_being_decoded_is_waited_for() {
        let decoder = Arc::new(Decoder::default());
        let claim = claim_free(&decoder.shared, key(1)).expect("free");
        let waiter = {
            let decoder = decoder.clone();
            std::thread::spawn(move || matches!(decoder.acquire("A.abk", 1), Found::Ready(_)))
        };
        std::thread::sleep(std::time::Duration::from_millis(20));
        claim.finish(Ok(decoded()));
        assert!(waiter.join().expect("thread"));
    }
}
