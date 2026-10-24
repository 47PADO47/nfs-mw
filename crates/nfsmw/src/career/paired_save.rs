//! Recoverable paired saves. The atomic native-profile replacement is the commit point.
use std::{fs, io::Write, path::Path};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::ProfileDir;

const JOURNAL: &str = "paired-save.json";

#[derive(Serialize, Deserialize)]
struct Pending {
    version: u8,
    extension: String,
    before_native: Option<String>,
    after_native: String,
    before: Option<Vec<u8>>,
    after: Vec<u8>,
}

fn optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("Cannot read {}", path.display())),
    }
}

fn fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    fs::create_dir_all(path.parent().context("Missing save folder")?)?;
    let temporary = path.with_extension("paired.tmp");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, path).with_context(|| format!("Cannot replace {}", path.display()))
}

fn restore(path: &Path, bytes: Option<&[u8]>) -> Result<()> {
    if let Some(bytes) = bytes {
        return replace(path, bytes);
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("Cannot remove an uncommitted save extension"),
    }
}

fn validate_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && name != JOURNAL && !name.contains(['/', '\\', ':']) && !matches!(name, "." | ".."),
        "Invalid paired save extension name"
    );
    Ok(())
}

pub(super) fn recover(profiles: &ProfileDir, alias: &str) -> Result<()> {
    let folder = profiles.mod_data_dir(alias)?;
    let file = folder.join(JOURNAL);
    let Some(bytes) = optional(&file)? else { return Ok(()) };
    let pending: Pending = serde_json::from_slice(&bytes).context("Invalid paired save journal")?;
    ensure!(pending.version == 1, "Unsupported paired save journal");
    validate_name(&pending.extension)?;
    let current = optional(&profiles.path(alias))?.as_deref().map(fingerprint);
    let target = folder.join(&pending.extension);
    if current.as_deref() == Some(pending.after_native.as_str()) {
        replace(&target, &pending.after)?;
        fs::remove_file(file)?;
        return Ok(());
    }
    // An external replacement must not inherit an uncommitted extension either.
    if current != pending.before_native {
        log::warn!("Profile changed outside the pending paired save; restoring its prior extension");
    }
    restore(&target, pending.before.as_deref())?;
    fs::remove_file(file)?;
    Ok(())
}

impl ProfileDir {
    pub(crate) fn recover_extensions(&self, alias: &str) -> Result<()> {
        recover(self, alias)
    }

    /// Stage a journal and extension, then commit the native profile atomically.
    pub(crate) fn write_with_extension(&self, alias: &str, native: &[u8], name: &str, extension: &[u8]) -> Result<()> {
        validate_name(name)?;
        recover(self, alias)?;
        let folder = self.mod_data_dir(alias)?;
        let target = folder.join(name);
        let pending = Pending {
            version: 1,
            extension: name.into(),
            before_native: optional(&self.path(alias))?.as_deref().map(fingerprint),
            after_native: fingerprint(native),
            before: optional(&target)?,
            after: extension.to_vec(),
        };
        let journal = folder.join(JOURNAL);
        replace(&journal, &serde_json::to_vec(&pending)?)?;
        let commit = replace(&target, extension).and_then(|()| self.write_native(alias, native));
        if let Err(error) = commit {
            // Keep the journal when rollback fails. The next read/write must recover it first.
            restore(&target, pending.before.as_deref()).context("Paired save rollback is pending recovery")?;
            fs::remove_file(&journal)?;
            return Err(error);
        }
        if let Err(error) = fs::remove_file(&journal) {
            // Both files committed. Reporting failure would make the caller roll back paid state.
            log::warn!("Paired save committed; journal cleanup will retry on the next read/write: {error}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
