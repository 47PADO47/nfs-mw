//! What the game knows of the rider's gear as a whole: the setting, the catalogue the mods offer, the choice the profile kept and
//! the one the garage shows before it is kept. One state for the process, since the rider is drawn by a world and by the garage's
//! backdrop at once; [`generation`] moves whenever any of it changes, so a reader looks again only then.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use anyhow::{Context, Result};
use game_install::GameDir;

use super::catalogue::{Catalogue, Part};
use super::choice::{self, Choice};

static ENABLED: AtomicBool = AtomicBool::new(true);
static GENERATION: AtomicU64 = AtomicU64::new(0);
static STATE: Mutex<State> = Mutex::new(State::new());

struct State {
    /// The choice the profile kept, and the file it was read from.
    persisted: Choice,
    loaded_from: Option<PathBuf>,
    /// A choice on show in the garage, not kept.
    preview: Option<Choice>,
    catalogue: Option<Arc<Catalogue>>,
}

impl State {
    const fn new() -> Self {
        Self { persisted: Choice::STOCK, loaded_from: None, preview: None, catalogue: None }
    }
}

fn lock() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

fn moved() {
    GENERATION.fetch_add(1, Ordering::Relaxed);
}

/// A number that changes whenever the setting, the catalogue, the kept choice or the previewed one does.
pub(crate) fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

/// Whether the setting `rider_gear` is on (it is by default).
pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// The setting as the host holds it now.
pub(crate) fn set_enabled(on: bool) {
    if ENABLED.swap(on, Ordering::Relaxed) != on {
        moved();
    }
}

/// Reads the install's catalogue (a mod's `RIDER/GEAR/gear.json`): the garage does it when it opens, so a mod that finished
/// converting meanwhile is seen.
pub(crate) fn scan(dir: &GameDir) {
    let found = Catalogue::load(dir).filter(|catalogue| !catalogue.is_empty()).map(Arc::new);
    let mut state = lock();
    let changed = state.catalogue.is_some() != found.is_some();
    state.catalogue = found;
    drop(state);
    if changed {
        moved();
    }
}

/// The catalogue the last [`scan`] found.
pub(crate) fn catalogue() -> Option<Arc<Catalogue>> {
    lock().catalogue.clone()
}

/// Whether there is anything to pick: the setting is on and a mod offers gear.
pub(crate) fn available() -> bool {
    enabled() && lock().catalogue.is_some()
}

/// Reads the profile's file again, when the profile or the file changed.
pub(crate) fn reload() {
    let path = choice::path();
    let fresh = path.as_deref().map(choice::read).unwrap_or_default();
    let mut state = lock();
    if state.persisted == fresh && state.loaded_from == path {
        return;
    }
    state.persisted = fresh;
    state.loaded_from = path;
    drop(state);
    moved();
}

/// What the profile kept, whatever is on show: the rider of a world wears this. The rider's own gear when the setting is off.
pub(crate) fn committed() -> Choice {
    if !enabled() {
        return Choice::default();
    }
    lock().persisted.clone()
}

/// What to draw now: the choice on show in the garage, else the kept one.
pub(crate) fn current() -> Choice {
    if !enabled() {
        return Choice::default();
    }
    let state = lock();
    state.preview.clone().unwrap_or_else(|| state.persisted.clone())
}

/// Shows `choice` in the garage's backdrop without keeping it; `None` takes the preview away.
pub(crate) fn set_preview(choice: Option<Choice>) {
    lock().preview = choice;
    moved();
}

/// Keeps `id` for `part` in the profile's file and takes the preview away.
pub(crate) fn commit(part: Part, id: &str) -> Result<()> {
    let path = choice::path().context("No profile folder for the rider's gear")?;
    let mut next = choice::read(&path);
    next.set(part, id);
    choice::write(&path, &next)?;
    let mut state = lock();
    state.persisted = next;
    state.loaded_from = Some(path);
    state.preview = None;
    drop(state);
    moved();
    Ok(())
}
