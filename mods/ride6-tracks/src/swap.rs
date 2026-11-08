//! Replace a generated folder with a finished one without needing the folder itself to go away.
//!
//! Windows refuses to delete or rename a folder another process has open (a file indexer, a virus scanner, an Explorer window,
//! a watcher): `remove_dir_all` then fails with "in use" (os error 32) and the promotion of a finished conversion is lost every
//! start. A folder that stays is harmless; what must change are the files in it. So the live folder keeps its place, its files
//! are removed one by one (best effort for the folders), and the finished run's files are moved into it.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Make `live` hold exactly what `staged` holds: the files of `live` go, the entries of `staged` are moved in.
pub fn replace_dir(live: &Path, staged: &Path) -> Result<()> {
    clear_contents(live)?;
    // A run that wrote nothing for this folder leaves it empty.
    if !staged.is_dir() {
        return Ok(());
    }
    move_into(staged, live)
}

/// Remove every file under `dir`, and the folders that can go; a folder that is in use stays (empty).
fn clear_contents(dir: &Path) -> Result<()> {
    let Ok(entries) = fs::read_dir(dir) else { return Ok(()) };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            clear_contents(&path)?;
            // A folder another process has open cannot be removed; it is emptied and reused.
            let _ = fs::remove_dir(&path);
            continue;
        }
        fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

/// Move the entries of `from` into `to`, merging with folders that already exist there and replacing files.
fn move_into(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).with_context(|| format!("creating {}", to.display()))?;
    for entry in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))?.flatten() {
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        if source.is_dir() {
            move_into(&source, &target)?;
            continue;
        }
        fs::rename(&source, &target).with_context(|| format!("moving {} into place", source.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_live_folder_takes_the_staged_files_and_loses_its_own() {
        let root = std::env::temp_dir().join(format!("ride6-tracks-swap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (live, staged) = (root.join("live"), root.join("staged"));
        fs::create_dir_all(live.join("a/b")).unwrap();
        fs::write(live.join("a/b/old.bin"), "old").unwrap();
        fs::write(live.join("same.bin"), "old").unwrap();
        fs::create_dir_all(staged.join("a/c")).unwrap();
        fs::write(staged.join("a/c/new.bin"), "new").unwrap();
        fs::write(staged.join("same.bin"), "new").unwrap();
        replace_dir(&live, &staged).unwrap();
        assert_eq!(fs::read_to_string(live.join("same.bin")).unwrap(), "new");
        assert_eq!(fs::read_to_string(live.join("a/c/new.bin")).unwrap(), "new");
        assert!(!live.join("a/b/old.bin").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_the_run_did_not_write_is_left_empty() {
        let root = std::env::temp_dir().join(format!("swap-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let live = root.join("live");
        fs::create_dir_all(live.join("a")).unwrap();
        fs::write(live.join("a/old.bin"), "old").unwrap();
        replace_dir(&live, &root.join("missing")).unwrap();
        assert!(!live.join("a/old.bin").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
