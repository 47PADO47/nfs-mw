//! The career at run time: the profile being played, its rules from `nfsmw-career` and the tables they need from
//! the install, shared between the front end and the world. Spec index: `docs/specs/career.md`.
//!
//! - [`data`]: the career tables read from the install ([`CareerData`]), converted to the rule crate's types.
//! - [`state`]: the [`Career`] being played: settings, stable, history, Blacklist, milestones and speed traps.
//! - [`io`]: the profile folder ([`ProfileDir`]): list, read and write profiles in the original's file format.
//! - [`start`]: Career on the main menu: resume the latest profile or start a new one.
//! - [`autosave`]: the autosave triggers (roaming start, event start, pursuit evaded).
//!
//! - [`sources`]: the profile list at the start: this build's folder plus, read only, the original game's save folder.
//!
//! The profiles are written to this build's own folder (`career` next to the config file), never to the original
//! game's `Documents\NFS Most Wanted`, so playing the rewrite cannot damage the original's saves. That folder is only
//! listed and read, so the original's saves can be picked at the start.

// The world and the screens that use most of this come in later milestones.
#![allow(dead_code)]

mod autosave;
mod card;
mod data;
mod io;
mod sources;
mod start;
mod state;

use std::sync::{Arc, Mutex, MutexGuard};

pub use autosave::{AutosaveInputs, handle_autosave};
pub use data::{CareerData, car_stats, event_id, race_event_id};
pub use io::ProfileDir;
pub use sources::Sources;
pub use state::Career;

/// What the career in the world asks the front end to do, taken once with `Scene::take_career_request`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CareerRequest {
    /// The player engaged a safe house: the world is left for its menu (`career-safehouse.md`).
    SafeHouse { zone: String },
    /// The player engaged a car lot: the car lot screen opens (`career-car-lot.md`).
    CarLot { zone: String },
    /// The player engaged an auto shop in career mode: the customization garage opens over the world, with no charge
    /// (the garage has no price data yet).
    AutoShop { zone: String },
    /// A stage that closes fewer gates was reached: the region unlock screen is due (`career-regions.md` section 6).
    RegionUnlock { stage: u8 },
    /// A text message was delivered in free roam: `SMS_Message.fng` opens over the frozen world at message `id`
    /// (`career-phone.md` section 3.6, `story-flow-2.md` section 3.7).
    Sms { id: u32 },
}

/// The profile the player picked and whether it is being played, shared by the front end, the world scene and the mod
/// backend. A profile is loaded from the alias list at the start (or by the alias manager) and stays loaded across the
/// menus; it is *played* only from the career menus on, so the quick race, free roam from the command line and every other
/// place that asks [`CareerHandle::with`] or [`CareerHandle::is_active`] sees no career. The menus that show the loaded
/// profile (the main menu card, the alias manager) use [`CareerHandle::with_loaded`].
#[derive(Clone, Default)]
pub struct CareerHandle(Arc<Mutex<Profile>>);

/// What a [`CareerHandle`] holds.
#[derive(Default)]
struct Profile {
    career: Option<Career>,
    playing: bool,
    /// The safe house menu stands before the world exists (Resume Career): the menus offer less there.
    prelude: bool,
}

impl CareerHandle {
    /// The process's career handle. The screens are built by name with no constructor data (`frontend::factory`), so
    /// they reach the career through this one handle, which the front end, the HUD and the world also hold.
    pub fn shared() -> CareerHandle {
        static SHARED: std::sync::OnceLock<CareerHandle> = std::sync::OnceLock::new();
        SHARED.get_or_init(CareerHandle::default).clone()
    }

    /// Loads `career` as the player's profile, replacing the one before. It is not played yet ([`Self::begin_play`]).
    pub fn load(&self, career: Career) {
        *self.lock() = Profile { career: Some(career), ..Profile::default() };
    }

    /// Starts playing `career`, replacing the one before.
    pub fn set(&self, career: Career) {
        *self.lock() = Profile { career: Some(career), playing: true, prelude: false };
    }

    /// Plays the loaded profile. Nothing happens when none is loaded.
    pub fn begin_play(&self) {
        let mut profile = self.lock();
        profile.playing = profile.career.is_some();
    }

    /// Leaves career mode; the profile stays loaded.
    pub fn end_play(&self) {
        let mut profile = self.lock();
        profile.playing = false;
        profile.prelude = false;
    }

    /// Marks the safe house menu as standing before the world (or not).
    pub fn set_prelude(&self, prelude: bool) {
        self.lock().prelude = prelude;
    }

    /// True while the safe house menu stands before the world exists.
    pub fn is_prelude(&self) -> bool {
        self.lock().prelude
    }

    /// Unloads the profile.
    pub fn clear(&self) {
        *self.lock() = Profile::default();
    }

    /// True while a career is being played.
    pub fn is_active(&self) -> bool {
        self.lock().playing
    }

    /// True while a profile is loaded, played or not.
    pub fn is_loaded(&self) -> bool {
        self.lock().career.is_some()
    }

    /// Runs `f` on the career, or returns `None` outside career mode.
    pub fn with<R>(&self, f: impl FnOnce(&mut Career) -> R) -> Option<R> {
        let mut profile = self.lock();
        if !profile.playing {
            return None;
        }
        profile.career.as_mut().map(f)
    }

    /// Runs `f` on the loaded profile, played or not, or returns `None` with no profile loaded.
    pub fn with_loaded<R>(&self, f: impl FnOnce(&mut Career) -> R) -> Option<R> {
        self.lock().career.as_mut().map(f)
    }

    fn lock(&self) -> MutexGuard<'_, Profile> {
        // A panic while the lock was held leaves the career as it was; keep using it.
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}
