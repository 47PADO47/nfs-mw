//! The profile folder: the career profiles this build writes and reads, in the original's file format.
//!
//! Layout: `<root>/<alias>/<alias>`, the same shape as `nfsmw_career::savefile::state::alias_file_path`. The root is
//! this build's own folder (`career` next to the config file, or `NFSMW_CAREER_DIR`), never the original's
//! `Documents\NFS Most Wanted`.
//!
//! Every read runs the load checks of `nfsmw_career::savefile::check_load`; every write is an atomic replace.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use nfsmw_career::save::layout::{PROFILE_AT, profile};
use nfsmw_career::savefile::check_load;
use nfsmw_career::savefile::state::{ALIAS_MAX_CHARS, is_listable_alias};

use super::mod_data::{ModData, flush};

/// The environment variable that overrides the profile folder.
const DIR_ENV: &str = "NFSMW_CAREER_DIR";
/// The folder name next to the config file.
const DIR_NAME: &str = "career";
/// The folder of a profile's mod data, next to its profile file (`docs/specs/mod-save-data.md`, section 2).
pub(super) const MOD_DATA_DIR: &str = "mods";

/// The profile folder of this build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileDir {
    root: PathBuf,
}

impl ProfileDir {
    /// A profile folder at `root`. Nothing is created until a write.
    pub fn new(root: impl Into<PathBuf>) -> ProfileDir {
        ProfileDir { root: root.into() }
    }

    /// The folder from `NFSMW_CAREER_DIR` when it is set and not empty, else `career` next to the config file.
    /// `None` when there is no per-user config folder.
    pub fn default_dir() -> Option<ProfileDir> {
        if let Some(dir) = std::env::var_os(DIR_ENV).filter(|dir| !dir.is_empty()) {
            return Some(ProfileDir::new(dir));
        }
        let config = crate::settings::Settings::config_path()?;
        Some(ProfileDir::new(config.parent()?.join(DIR_NAME)))
    }

    /// The root folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The profile file of `alias`: `<root>/<alias>/<alias>`. Does not check the alias; [`read`](Self::read),
    /// [`write`](Self::write) and [`remove`](Self::remove) do, and only they touch the disk.
    pub fn path(&self, alias: &str) -> PathBuf {
        self.root.join(alias).join(alias)
    }

    /// The aliases with a profile file: folders holding a file of the same name that the list shows (at most 31
    /// characters, no space), sorted. An unreadable or missing root gives none.
    pub fn aliases(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(&self.root) else { return Vec::new() };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| self.has_profile(name))
            .collect();
        names.sort();
        names
    }

    /// The alias whose profile file was modified last, `None` when there is no profile.
    pub fn latest(&self) -> Option<String> {
        self.aliases()
            .into_iter()
            .filter_map(|alias| {
                let modified = fs::metadata(self.path(&alias)).ok()?.modified().ok()?;
                Some((modified, alias))
            })
            .max()
            .map(|(_, alias)| alias)
    }

    /// Reads the profile of `alias` and runs the load checks on it. A refused file is an error carrying the
    /// check's report.
    pub fn read(&self, alias: &str) -> Result<Vec<u8>> {
        let file = self.file_of(alias)?;
        let bytes = fs::read(&file).with_context(|| format!("cannot read profile {}", file.display()))?;
        match check_load(&bytes, stored_stable_hash(&bytes)) {
            Ok(_) => Ok(bytes),
            Err(report) => bail!("profile {alias} refused: {report}"),
        }
    }

    /// Writes the profile of `alias`, creating its folder. The bytes go to `<file>.tmp` first and are then renamed
    /// over the profile, so a failed write leaves the old file in place. The bytes are not checked here.
    pub fn write(&self, alias: &str, bytes: &[u8]) -> Result<()> {
        let file = self.file_of(alias)?;
        let dir = file.parent().context("the profile has no folder")?;
        fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
        let tmp = PathBuf::from(format!("{}.tmp", file.display()));
        fs::write(&tmp, bytes).with_context(|| format!("cannot write {}", tmp.display()))?;
        let renamed = fs::rename(&tmp, &file);
        if renamed.is_err() {
            // Best effort: the temporary file is not the profile, and the rename error is the one to report.
            let _ = fs::remove_file(&tmp);
        }
        renamed.with_context(|| format!("cannot replace {}", file.display()))
    }

    /// Saves the profile of `alias` with `bytes`. The mod data staged for the profile is written first, then the profile
    /// (`docs/specs/mod-save-data.md`, section 5). When a mod write fails, the profile is not written.
    pub fn save(&self, alias: &str, bytes: &[u8]) -> Result<()> {
        let blobs = ModData::new(self.mod_data_dir(alias)?);
        flush(&blobs, alias).with_context(|| format!("the mod data of {alias} could not be saved"))?;
        self.write(alias, bytes)
    }

    /// The folder that holds the mod data of `alias`: `<root>/<alias>/mods` (`docs/specs/mod-save-data.md`, section 2).
    /// Does not create it, and does not check that it exists.
    pub fn mod_data_dir(&self, alias: &str) -> Result<PathBuf> {
        let file = self.file_of(alias)?;
        let dir = file.parent().context("the profile has no folder")?;
        Ok(dir.join(MOD_DATA_DIR))
    }

    /// Deletes the profile of `alias`, its mod data and its folder. The folder goes only when nothing else is left in
    /// it. A missing profile is an error.
    pub fn remove(&self, alias: &str) -> Result<()> {
        let file = self.file_of(alias)?;
        fs::remove_file(&file).with_context(|| format!("cannot delete {}", file.display()))?;
        let Some(dir) = file.parent() else { return Ok(()) };
        let mods = dir.join(MOD_DATA_DIR);
        if mods.exists() {
            fs::remove_dir_all(&mods).with_context(|| format!("cannot delete {}", mods.display()))?;
        }
        // Fails, and is ignored, when the folder still holds other files.
        let _ = fs::remove_dir(dir);
        Ok(())
    }

    /// True when `alias` is listable and has a profile file.
    fn has_profile(&self, alias: &str) -> bool {
        is_listable_alias(alias) && self.file_of(alias).is_ok_and(|file| file.is_file())
    }

    /// The profile file of a valid alias, or the reason it is refused.
    fn file_of(&self, alias: &str) -> Result<PathBuf> {
        check_alias(alias)?;
        Ok(self.path(alias))
    }
}

/// Refuses an alias that could reach outside the root or that the library cannot keep: empty, a path separator
/// or drive colon, `..`, or longer than 31 characters (the library cuts a longer alias, which would make two
/// names share one file; `docs/specs/career-save-screens.md`, the alias is cut to 31 characters).
fn check_alias(alias: &str) -> Result<()> {
    if alias.is_empty() {
        bail!("the alias is empty");
    }
    if alias.contains(['/', '\\', ':', '\0']) {
        bail!("the alias {alias:?} has a path separator or colon");
    }
    if alias.contains("..") {
        bail!("the alias {alias:?} contains ..");
    }
    if alias.chars().count() > ALIAS_MAX_CHARS {
        bail!("the alias {alias:?} is longer than {ALIAS_MAX_CHARS} characters");
    }
    Ok(())
}

/// The default stable hash the file stores (`profile::DEFAULT_STABLE_HASH` in the profile buffer). A file too short
/// to hold it gives 0; `check_load` refuses such a file before it compares the hash.
fn stored_stable_hash(bytes: &[u8]) -> u32 {
    let at = PROFILE_AT + profile::DEFAULT_STABLE_HASH;
    match bytes.get(at..at + 4) {
        Some(word) => u32::from_le_bytes([word[0], word[1], word[2], word[3]]),
        None => 0,
    }
}

#[cfg(test)]
#[path = "io_tests.rs"]
mod tests;
