//! The career at run time: the profile being played, its rules from `nfsmw-career` and the tables they need from
//! the install, shared between the front end and the world. Spec index: `docs/specs/career.md`.
//!
//! - [`data`]: the career tables read from the install ([`CareerData`]), converted to the rule crate's types.
//! - [`state`]: the [`Career`] being played: settings, stable, history, Blacklist, milestones and speed traps.
//! - [`io`]: the profile folder ([`ProfileDir`]): list, read and write profiles in the original's file format.
//! - [`start`]: Career on the main menu: resume the latest profile or start a new one.
//! - [`autosave`]: the autosave triggers (roaming start, event start, pursuit evaded).
//!
//! The profiles are written to this build's own folder (`career` next to the config file), never to the original
//! game's `Documents\NFS Most Wanted`, so playing the rewrite cannot damage the original's saves.

// The world and the screens that use most of this come in later milestones.
#![allow(dead_code)]

mod autosave;
mod data;
mod io;
mod start;
mod state;

use std::sync::{Arc, Mutex, MutexGuard};

pub use data::CareerData;
pub use io::ProfileDir;
pub use start::start_or_resume;
pub use state::Career;

/// The career being played, shared by the front end, the world scene and the mod backend. Empty outside career
/// mode (quick race, free roam from the command line).
#[derive(Clone, Default)]
pub struct CareerHandle(Arc<Mutex<Option<Career>>>);

impl CareerHandle {
    /// Starts playing `career`, replacing the one before.
    pub fn set(&self, career: Career) {
        *self.lock() = Some(career);
    }

    /// Leaves career mode.
    pub fn clear(&self) {
        *self.lock() = None;
    }

    /// True while a career is being played.
    pub fn is_active(&self) -> bool {
        self.lock().is_some()
    }

    /// Runs `f` on the career, or returns `None` outside career mode.
    pub fn with<R>(&self, f: impl FnOnce(&mut Career) -> R) -> Option<R> {
        self.lock().as_mut().map(f)
    }

    fn lock(&self) -> MutexGuard<'_, Option<Career>> {
        // A panic while the lock was held leaves the career as it was; keep using it.
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}
