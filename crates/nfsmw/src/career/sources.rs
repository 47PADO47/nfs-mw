//! Where the profile list comes from: this build's own profile folder and, read only, the original game's save folder
//! (`Documents\NFS Most Wanted`, one `<alias>\<alias>` file per profile, the same layout). The original's own list at the
//! start shows its saves, so this build shows them too, and a pick loads one. Nothing is ever written to the original's
//! folder: a loaded profile is saved to this build's folder, where it then hides the original's copy of the same alias
//! (`docs/specs/boot-profile-prompt.md`, section 4).

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;

use super::ProfileDir;

/// The environment variable that names the original game's save folder. An empty value turns the original's saves off.
const ORIGINAL_ENV: &str = "NFSMW_ORIGINAL_SAVES_DIR";
/// The original's save folder under the user's profile folder.
const ORIGINAL_FOLDER: [&str; 2] = ["Documents", "NFS Most Wanted"];

/// The folders profiles are listed and read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sources {
    /// This build's folder: read and written.
    own: Option<ProfileDir>,
    /// The original's folder: read only.
    original: Option<ProfileDir>,
}

impl Sources {
    pub fn new(own: Option<ProfileDir>, original: Option<ProfileDir>) -> Sources {
        Sources { own, original }
    }

    /// This build's folder (`ProfileDir::default_dir`) and the original's ([`original_dir`]).
    pub fn standard() -> Sources {
        Sources::new(ProfileDir::default_dir(), original_dir())
    }

    /// `own` (the folder the front end saves to) and the original's folder.
    pub fn with_own(own: Option<ProfileDir>) -> Sources {
        Sources::new(own, original_dir())
    }

    /// The aliases of both folders, each once (an alias in both counts once, compared without case as the Windows folders
    /// are), sorted without case.
    pub fn aliases(&self) -> Vec<String> {
        let mut names: Vec<String> = self.own.iter().flat_map(ProfileDir::aliases).collect();
        for alias in self.original.iter().flat_map(ProfileDir::aliases) {
            if !names.iter().any(|name| name.eq_ignore_ascii_case(&alias)) {
                names.push(alias);
            }
        }
        names.sort_by_key(|name| name.to_ascii_lowercase());
        names
    }

    /// True when neither folder holds a profile.
    pub fn is_empty(&self) -> bool {
        self.aliases().is_empty()
    }

    /// Reads the profile of `alias`: this build's copy when it has one, else the original's. Either goes through the load
    /// checks of [`ProfileDir::read`].
    pub fn read(&self, alias: &str) -> Result<Vec<u8>> {
        if let Some(own) = &self.own
            && own.aliases().iter().any(|a| a == alias)
        {
            return own.read(alias);
        }
        if let Some(original) = &self.original {
            return original.read(alias);
        }
        match &self.own {
            Some(own) => own.read(alias),
            None => anyhow::bail!("there is no profile folder to read {alias} from"),
        }
    }
}

/// The original game's save folder: `NFSMW_ORIGINAL_SAVES_DIR` when it is set (empty means none), else
/// `Documents\NFS Most Wanted` under the user's profile folder. `None` when there is none to look in.
pub fn original_dir() -> Option<ProfileDir> {
    let given = std::env::var_os(ORIGINAL_ENV);
    let user = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    original_dir_from(given, user)
}

/// [`original_dir`] over the two values it reads, so a test can give them.
fn original_dir_from(given: Option<OsString>, user: Option<OsString>) -> Option<ProfileDir> {
    if let Some(dir) = given {
        if dir.is_empty() {
            return None;
        }
        return Some(ProfileDir::new(dir));
    }
    let user = user.filter(|user| !user.is_empty())?;
    let mut path = PathBuf::from(user);
    path.extend(ORIGINAL_FOLDER);
    Some(ProfileDir::new(path))
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;
