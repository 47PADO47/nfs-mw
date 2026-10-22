//! `bb.saves`: a mod's own blob in the career profile loaded now (`docs/specs/mod-save-data.md`).
//!
//! A write or a removal is staged, and the profile's next save writes it before the profile itself. A read sees a staged
//! change first. With no profile loaded every call is `BB_ERR_NOT_READY`: there is no profile to keep the blob in.

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_CORRUPT, BB_ERR_HOST, BB_ERR_NOT_FOUND, BB_ERR_NOT_READY, BbStatus};
use blackbox_modding::host::{BackendResult, SavesBackend};

use crate::career::mod_data::{self, ModData, ModDataError};
use crate::career::{CareerHandle, ProfileDir};

/// The `bb.saves` port of the game: the blobs of the career profile loaded now.
pub struct SavesPort {
    profiles: Option<ProfileDir>,
    /// The alias of the loaded profile, `None` when none is loaded.
    loaded: Box<dyn Fn() -> Option<String>>,
}

impl SavesPort {
    /// The port over this build's profile folder `profiles`, for the profile the career handle has loaded.
    pub fn new(profiles: Option<ProfileDir>) -> SavesPort {
        SavesPort::with_loader(profiles, Box::new(|| CareerHandle::shared().with_loaded(|career| career.alias.clone())))
    }

    /// The port over `profiles`, with `loaded` naming the loaded profile. The tests give their own.
    pub(super) fn with_loader(profiles: Option<ProfileDir>, loaded: Box<dyn Fn() -> Option<String>>) -> SavesPort {
        SavesPort { profiles, loaded }
    }

    /// The alias of the loaded profile.
    fn alias(&self) -> BackendResult<String> {
        (self.loaded)().ok_or(BB_ERR_NOT_READY)
    }

    /// The blob folder of the profile `alias`.
    fn blobs(&self, alias: &str) -> BackendResult<ModData> {
        let profiles = self.profiles.as_ref().ok_or(BB_ERR_NOT_READY)?;
        let dir = profiles.mod_data_dir(alias).map_err(|_| BB_ERR_NOT_READY)?;
        Ok(ModData::new(dir))
    }
}

impl SavesBackend for SavesPort {
    fn read(&self, owner: &str) -> BackendResult<Vec<u8>> {
        let alias = self.alias()?;
        match mod_data::staged_for(&alias, owner) {
            Some(Some(body)) => return Ok(body),
            Some(None) => return Err(BB_ERR_NOT_FOUND),
            None => {}
        }
        let blobs = self.blobs(&alias)?;
        match blobs.read(owner) {
            Ok(Some(body)) => Ok(body),
            Ok(None) => Err(BB_ERR_NOT_FOUND),
            Err(error) => Err(status_of(&error)),
        }
    }

    fn write(&self, owner: &str, body: &[u8]) -> BackendResult<()> {
        let alias = self.alias()?;
        mod_data::stage(&alias, owner, Some(body.to_vec())).map_err(|error| status_of(&error))
    }

    fn remove(&self, owner: &str) -> BackendResult<()> {
        let alias = self.alias()?;
        mod_data::stage(&alias, owner, None).map_err(|error| status_of(&error))
    }
}

/// The status a mod data failure reports: a bad key or body is `BB_ERR_ARG`, a corrupt blob `BB_ERR_CORRUPT`, and a
/// file that cannot be read or written `BB_ERR_HOST`.
fn status_of(error: &ModDataError) -> BbStatus {
    match error {
        ModDataError::BadKey(_) | ModDataError::TooLarge(_) => BB_ERR_ARG,
        ModDataError::Corrupt => BB_ERR_CORRUPT,
        ModDataError::Io(..) => BB_ERR_HOST,
    }
}
