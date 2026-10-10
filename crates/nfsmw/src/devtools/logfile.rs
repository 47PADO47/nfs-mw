//! The log file: every log line, saved in the per-user log folder ([`game_install::log_dir`]). Each start clears the
//! logs of the earlier runs, so the folder holds only the run in progress.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

/// The file this run writes to. The earlier runs' files are cleared before it is created.
const FILE_NAME: &str = "nfsmw.log";

/// Clears the earlier logs and creates this run's log file. `None` when there is no log folder or it cannot be
/// written: logging is not up yet, so the reason goes to stderr and the game still starts.
pub fn open() -> Option<(PathBuf, File)> {
    let Some(dir) = game_install::log_dir(&nfsmw_data::game::SPEC) else {
        eprintln!("there is no per-user log folder; the log is not saved to a file");
        return None;
    };
    match open_in(&dir) {
        Ok(file) => Some((dir.join(FILE_NAME), file)),
        Err(e) => {
            eprintln!("could not create the log file in {}: {e}", dir.display());
            None
        }
    }
}

/// Clears `dir` of the earlier logs, then creates this run's log file in it.
fn open_in(dir: &Path) -> io::Result<File> {
    fs::create_dir_all(dir)?;
    clear_logs(dir)?;
    File::create(dir.join(FILE_NAME))
}

/// Deletes the `.log` files in `dir`. Other files are left alone, and a file that cannot be deleted (another run
/// still has it open) is reported and kept.
fn clear_logs(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if !(path.is_file() && path.extension().is_some_and(|ext| ext == "log")) {
            continue;
        }
        if let Err(e) = fs::remove_file(&path) {
            eprintln!("could not clear the old log {}: {e}", path.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder for one test, emptied first so an earlier failed run cannot leak files into it.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nfsmw-logfile-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_new_run_clears_the_earlier_logs_and_keeps_other_files() {
        let dir = scratch("clears");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("nfsmw.log"), "last run").unwrap();
        fs::write(dir.join("older.log"), "older").unwrap();
        fs::write(dir.join("notes.txt"), "keep me").unwrap();

        let _file = open_in(&dir).unwrap();

        assert!(!dir.join("older.log").exists());
        assert_eq!(fs::read_to_string(dir.join("nfsmw.log")).unwrap(), "", "this run starts empty");
        assert_eq!(fs::read_to_string(dir.join("notes.txt")).unwrap(), "keep me");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_folder_is_created() {
        let dir = scratch("creates").join("nested");
        let _file = open_in(&dir).unwrap();
        assert!(dir.join(FILE_NAME).is_file());
        let _ = fs::remove_dir_all(dir.parent().unwrap());
    }
}
