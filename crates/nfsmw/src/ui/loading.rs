//! The UI assets read on a background thread, so the window opens and answers while they load. The front end and the HUD
//! build their own parts on threads that wait for the assets ([`AssetsLoad::wait`]); the systems that draw with them run
//! once their resources exist (`run_if`). While any part is still loading a screenshot waits and a benchmark does not
//! start ([`UiLoading`], `Host::ui_loading`).

use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Instant;

use bevy_ecs::prelude::*;
use game_install::GameDir;

use super::fonts::{CustomFontSet, scan};
use super::{SharedAssets, UiAssets};
use crate::app::Host;

/// The name of the shared assets' load in [`UiLoading`].
pub(super) const ASSETS: &str = "the UI assets";

/// What the load gave: the assets, or `None` when the install cannot supply them (the UI is off).
type Outcome = Option<Arc<UiAssets>>;

/// The UI assets as they load. The loader threads hold a clone and wait on it; the main thread only polls it.
#[derive(Resource, Clone, Default)]
pub struct AssetsLoad(Arc<(Mutex<Option<Outcome>>, Condvar)>);

impl AssetsLoad {
    /// Starts reading the assets of `dir` on a background thread.
    pub fn start(dir: GameDir) -> Self {
        let load = Self::default();
        let out = load.clone();
        let spawned = std::thread::Builder::new().name("ui-assets".into()).spawn(move || {
            let read = std::panic::catch_unwind(AssertUnwindSafe(|| read(&dir)));
            let outcome = read.unwrap_or_else(|_| {
                log::error!("the UI is off: reading its assets panicked");
                None
            });
            out.finish(outcome);
        });
        if let Err(e) = spawned {
            log::warn!("the UI is off: cannot start the thread that reads its assets: {e}");
            load.finish(None);
        }
        load
    }

    /// A load that is already over (tests).
    #[cfg(test)]
    pub fn done(assets: Outcome) -> Self {
        let load = Self::default();
        load.finish(assets);
        load
    }

    fn finish(&self, assets: Outcome) {
        let (slot, ready) = &*self.0;
        *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(assets);
        ready.notify_all();
    }

    /// What the load gave, `None` while it still runs. Never waits.
    pub fn poll(&self) -> Option<Outcome> {
        self.0.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Waits until the load is over. For loader threads: the main thread polls instead.
    pub fn wait(&self) -> Outcome {
        let (slot, ready) = &*self.0;
        let guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
        let guard = ready.wait_while(guard, |outcome| outcome.is_none()).unwrap_or_else(PoisonError::into_inner);
        guard.clone().flatten()
    }
}

/// Reads the assets, with the custom fonts: they are read whether or not they are on, so the console can switch them on
/// without a restart.
fn read(dir: &GameDir) -> Outcome {
    let custom =
        crate::settings::custom_fonts::folder().map(|folder| CustomFontSet::load(&scan(&folder))).unwrap_or_default();
    match UiAssets::load_with(dir, custom) {
        Ok(assets) => Some(Arc::new(assets)),
        Err(e) => {
            log::warn!("the UI is off: {e:#}");
            None
        }
    }
}

/// Work on a named background thread whose result the main thread picks up without waiting. Work that cannot start or
/// that panics gives `T::default()`, so the part it builds is simply off.
pub struct Job<T>(Option<JoinHandle<T>>);

impl<T: Default + Send + 'static> Job<T> {
    pub fn spawn(name: &str, work: impl FnOnce() -> T + Send + 'static) -> Self {
        match std::thread::Builder::new().name(name.into()).spawn(work) {
            Ok(handle) => Self(Some(handle)),
            Err(e) => {
                log::warn!("cannot start the {name} thread: {e}");
                Self(None)
            }
        }
    }

    /// The result once the work is over, `None` while it runs. Taken once: after that it is `T::default()`.
    pub fn take(&mut self) -> Option<T> {
        let Some(handle) = self.0.take_if(|handle| handle.is_finished()) else {
            return self.0.is_none().then(T::default);
        };
        let name = handle.thread().name().unwrap_or("loader").to_owned();
        Some(handle.join().unwrap_or_else(|_| {
            log::error!("the {name} thread panicked");
            T::default()
        }))
    }
}

/// The UI parts still loading, by name, with when they started.
#[derive(Resource, Default)]
pub struct UiLoading(Vec<(&'static str, Instant)>);

impl UiLoading {
    pub fn begin(&mut self, part: &'static str) {
        if self.0.iter().any(|(name, _)| *name == part) {
            return;
        }
        self.0.push((part, Instant::now()));
    }

    pub fn end(&mut self, part: &'static str) {
        let Some(at) = self.0.iter().position(|(name, _)| *name == part) else { return };
        let (_, started) = self.0.remove(at);
        log::info!("{part} loaded in {} ms", started.elapsed().as_millis());
    }

    /// Whether any part is still loading.
    pub fn busy(&self) -> bool {
        !self.0.is_empty()
    }
}

/// Inserts [`SharedAssets`] once the load is over. A failed load inserts nothing: the UI is off, as it always was.
pub(super) fn publish_assets(
    mut commands: Commands,
    load: Res<AssetsLoad>,
    mut loading: ResMut<UiLoading>,
    mut over: Local<bool>,
) {
    if *over {
        return;
    }
    let Some(outcome) = load.poll() else { return };
    *over = true;
    loading.end(ASSETS);
    let Some(assets) = outcome else { return };
    commands.insert_resource(SharedAssets(assets));
}

/// Tells the host whether UI parts are still loading: a screenshot waits for them and a benchmark starts after them.
pub(super) fn tell_host(loading: Res<UiLoading>, mut host: NonSendMut<Host>) {
    let busy = loading.busy();
    if host.ui_loading != busy {
        host.ui_loading = busy;
    }
}

#[cfg(test)]
#[path = "loading_tests.rs"]
mod tests;
