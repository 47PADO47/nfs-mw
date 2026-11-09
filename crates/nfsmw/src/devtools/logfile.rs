//! The log file: every log line, saved in the per-user log folder ([`game_install::log_dir`]). A start keeps the logs of
//! the last few runs (`nfsmw.1.log` is the run before this one) and clears the older ones, so a later start does not
//! destroy the log of the run a player is about to read.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

/// The file this run writes to.
const FILE_NAME: &str = "nfsmw.log";

/// How many earlier runs keep their log: `nfsmw.1.log` is the newest of them, `nfsmw.3.log` the oldest. Three, so a few
/// short starts (a check, a screenshot) in a row do not push the log of a real session out of the folder.
const KEEP_PREVIOUS: usize = 3;

/// The file of the run `age` runs before this one (`1` is the run just before).
fn previous_name(age: usize) -> String {
    format!("nfsmw.{age}.log")
}

/// Rotates the earlier logs and creates this run's log file. `None` when there is no log folder or it cannot be
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

/// Rotates the logs in `dir`, then creates this run's log file in it.
fn open_in(dir: &Path) -> io::Result<File> {
    fs::create_dir_all(dir)?;
    rotate(dir)?;
    File::create(dir.join(FILE_NAME))
}

/// Whether `name` is one of the files this module writes (`nfsmw.log`, `nfsmw.N.log`).
fn is_rotated(name: &str) -> bool {
    name == FILE_NAME || (1..=KEEP_PREVIOUS).any(|age| name == previous_name(age))
}

/// Deletes the other `.log` files in `dir`, which belong to no run this module knows, then moves each kept log one
/// step older and the last run's log to `nfsmw.1.log`. A last run that wrote nothing is not kept (it would push a log
/// worth reading out). Other files are left alone, and a file that cannot be moved or deleted (another run still has it
/// open) is reported and kept.
fn rotate(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        let stray = path.is_file() && path.extension().is_some_and(|ext| ext == "log") && !is_rotated(name);
        if !stray {
            continue;
        }
        if let Err(e) = fs::remove_file(&path) {
            eprintln!("could not clear the old log {}: {e}", path.display());
        }
    }
    let last = dir.join(FILE_NAME);
    let wrote_something = fs::metadata(&last).is_ok_and(|meta| meta.len() > 0);
    if !wrote_something {
        return Ok(());
    }
    // The oldest goes first; each log then takes the place of the one older than it.
    let _ = fs::remove_file(dir.join(previous_name(KEEP_PREVIOUS)));
    for age in (1..KEEP_PREVIOUS).rev() {
        let from = dir.join(previous_name(age));
        if !from.is_file() {
            continue;
        }
        if let Err(e) = fs::rename(&from, dir.join(previous_name(age + 1))) {
            eprintln!("could not keep the old log {}: {e}", from.display());
        }
    }
    if let Err(e) = fs::rename(&last, dir.join(previous_name(1))) {
        eprintln!("could not keep the last log {}: {e}", last.display());
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
    fn a_new_run_keeps_the_last_log_and_clears_the_strays() {
        let dir = scratch("keeps");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("nfsmw.log"), "last run").unwrap();
        fs::write(dir.join("older.log"), "older").unwrap();
        fs::write(dir.join("notes.txt"), "keep me").unwrap();

        let _file = open_in(&dir).unwrap();

        assert!(!dir.join("older.log").exists(), "a log of no run this module knows is cleared");
        assert_eq!(fs::read_to_string(dir.join("nfsmw.1.log")).unwrap(), "last run");
        assert_eq!(fs::read_to_string(dir.join("nfsmw.log")).unwrap(), "", "this run starts empty");
        assert_eq!(fs::read_to_string(dir.join("notes.txt")).unwrap(), "keep me");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_logs_age_one_step_a_run_and_the_oldest_goes() {
        let dir = scratch("ages");
        fs::create_dir_all(&dir).unwrap();
        for (name, text) in [("nfsmw.log", "d"), ("nfsmw.1.log", "c"), ("nfsmw.2.log", "b"), ("nfsmw.3.log", "a")] {
            fs::write(dir.join(name), text).unwrap();
        }

        let _file = open_in(&dir).unwrap();

        let read = |name: &str| fs::read_to_string(dir.join(name)).unwrap();
        assert_eq!(
            (read("nfsmw.1.log"), read("nfsmw.2.log"), read("nfsmw.3.log")),
            ("d".into(), "c".into(), "b".into())
        );
        assert_eq!(read("nfsmw.log"), "");
        assert!(!dir.join("nfsmw.4.log").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_last_run_that_wrote_nothing_does_not_push_a_log_out() {
        let dir = scratch("empty");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("nfsmw.log"), "").unwrap();
        fs::write(dir.join("nfsmw.1.log"), "worth reading").unwrap();

        let _file = open_in(&dir).unwrap();

        assert_eq!(fs::read_to_string(dir.join("nfsmw.1.log")).unwrap(), "worth reading");
        assert!(!dir.join("nfsmw.2.log").exists());
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
