//! The import as a background job, and how its output reaches the mod's folder.
//!
//! The mod folder holds what the game reads (`files/`, `worlds/`, and the `.import-stamp` of the run that made them). A run
//! never writes there: it builds in `.staging/` and the finished result is *promoted* by renaming. The game has indexed the
//! folder when it started, so a run that finishes while content is already live is promoted at the next start, and a first
//! run, which has nothing live to disturb, is promoted at once and announced to the game. (The same scheme as
//! `mods/ride6-bikes/src/job.rs`.)

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};

use crate::cells::{Keep, Limits};
use crate::config::{self, Config};
use crate::import::{self, Report};
use crate::stamp;

const STAGING: &str = ".staging";
/// The folders the game reads, which a promotion replaces together.
const CONTENT: [&str; 2] = ["files", "worlds"];
/// The import's thread builds large vectors and recurses a little; a roomy stack costs nothing.
const STACK_BYTES: usize = 64 * 1024 * 1024;

/// What a run did: one report per world, and the packs that could not be imported with the reason.
#[derive(Debug, Default)]
pub struct Summary {
    pub imported: Vec<Report>,
    pub failed: Vec<(String, String)>,
}

/// An import running on its own thread.
pub struct Job {
    receiver: Receiver<Result<Summary>>,
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// What the mod found at start-up.
pub enum Start {
    /// No `source` is set. `created` says the settings file was missing and has just been written.
    NotConfigured { config: PathBuf, created: bool },
    /// The content on disk is right for the settings and the inputs.
    UpToDate { worlds: usize },
    /// An import was started; `live` says the folder already holds content the game has indexed.
    Started { job: Job, live: bool },
}

/// What a job did, once it is over.
pub enum Finished {
    Done(Summary),
    Failed(String),
}

/// The game unloads the mod's library when it closes, and a thread still running in it would then run unloaded code: the
/// job is stopped before its next cell and waited for.
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Job {
    /// The result when the job is over, `None` while it runs.
    pub fn poll(&self) -> Option<Finished> {
        match self.receiver.try_recv() {
            Ok(Ok(summary)) => Some(Finished::Done(summary)),
            Ok(Err(error)) => Some(Finished::Failed(format!("{error:#}"))),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Finished::Failed("the import thread stopped".to_owned())),
        }
    }
}

/// What the mod does at load: promote a finished run left by an earlier start, take the settings the host merged from
/// `mod_dir/config.toml` and the game's `[mods.ride6-tracks]` table (`supplied`; without it, read `config.toml` alone), and,
/// unless the content on disk is still right, start importing in the background.
pub fn start(mod_dir: &Path, supplied: Option<&config::Supplied>) -> Result<Start> {
    promote(mod_dir)?;
    let config_path = mod_dir.join(config::FILE_NAME);
    let (config, created) = Config::resolve(supplied, &config_path)?;
    let Some(source) = config.source.clone() else {
        return Ok(Start::NotConfigured { config: config_path, created });
    };
    let packs = stamp::discover(&source);
    let stamp = stamp::of(&config, &packs);
    if stamp::is_current(mod_dir, &stamp) {
        return Ok(Start::UpToDate { worlds: count_worlds(mod_dir) });
    }
    let live = mod_dir.join(CONTENT[0]).is_dir();
    let keep = Keep { objects: config.objects, line: config.racing_line, brightness: config.brightness };
    let job = spawn((config.max_texture, keep), packs, mod_dir.to_owned(), stamp)?;
    Ok(Start::Started { job, live })
}

fn spawn(options: (u32, Keep), packs: Vec<PathBuf>, mod_dir: PathBuf, stamp: String) -> Result<Job> {
    let (sender, receiver) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&cancel);
    let thread = thread::Builder::new()
        .name(config::MOD_ID.to_owned())
        .stack_size(STACK_BYTES)
        .spawn(move || {
            let staging = mod_dir.join(STAGING);
            let _ = fs::remove_dir_all(&staging);
            let result = build(options, &packs, &staging, &stamp, &stop);
            if result.is_err() {
                let _ = fs::remove_dir_all(&staging);
            }
            let _ = sender.send(result);
        })
        .context("starting the import thread")?;
    Ok(Job { receiver, cancel, thread: Some(thread) })
}

fn build(
    (max_texture, keep): (u32, Keep),
    packs: &[PathBuf],
    staging: &Path,
    stamp: &str,
    cancel: &AtomicBool,
) -> Result<Summary> {
    fs::create_dir_all(staging).with_context(|| format!("creating {}", staging.display()))?;
    let mut summary = Summary::default();
    for pack in packs {
        match import::import_with(pack, staging, max_texture, &Limits::default(), &keep, cancel) {
            Ok(report) => summary.imported.push(report),
            Err(error) if cancel.load(Ordering::Relaxed) => return Err(error),
            Err(error) => summary.failed.push((pack.display().to_string(), format!("{error:#}"))),
        }
    }
    // The stamp is written last: a staging folder without one was cut short and is thrown away.
    stamp::save(staging, stamp).context("saving the import stamp")?;
    Ok(summary)
}

/// Replace the folders the game reads with the finished run in `.staging/`, if there is one. A staging folder that was not
/// finished is deleted. Returns whether anything was promoted.
pub fn promote(mod_dir: &Path) -> Result<bool> {
    let staging = mod_dir.join(STAGING);
    if !staging.is_dir() {
        return Ok(false);
    }
    if !staging.join(stamp::FILE).is_file() {
        fs::remove_dir_all(&staging).with_context(|| format!("removing {}", staging.display()))?;
        return Ok(false);
    }
    stamp::clear(mod_dir);
    for part in CONTENT {
        let live = mod_dir.join(part);
        if live.exists() {
            fs::remove_dir_all(&live).with_context(|| format!("clearing {}", live.display()))?;
        }
        let staged = staging.join(part);
        if staged.exists() {
            fs::rename(&staged, &live).with_context(|| format!("moving {} into place", staged.display()))?;
        }
    }
    fs::rename(staging.join(stamp::FILE), mod_dir.join(stamp::FILE)).context("moving the import stamp into place")?;
    // The content is in place; a staging folder that cannot be removed (a scanner holds it) is cleaned at the next start.
    if let Err(error) = fs::remove_dir_all(&staging) {
        log::warn!("ride6-tracks: the staging folder is not removed: {error}");
    }
    Ok(true)
}

fn count_worlds(mod_dir: &Path) -> usize {
    let worlds = fs::read_dir(mod_dir.join("worlds")).into_iter().flatten().flatten();
    worlds.filter(|entry| entry.path().extension().is_some_and(|ext| ext == "toml")).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-tracks-job-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn staged(dir: &Path, finished: bool) {
        fs::create_dir_all(dir.join(".staging/files/WORLDS/X")).unwrap();
        fs::create_dir_all(dir.join(".staging/worlds")).unwrap();
        fs::write(dir.join(".staging/worlds/x.toml"), "new").unwrap();
        if finished {
            fs::write(dir.join(".staging").join(stamp::FILE), "stamp").unwrap();
        }
    }

    #[test]
    fn a_finished_run_replaces_the_content_and_keeps_the_rest() {
        let dir = temp("promote");
        fs::create_dir_all(dir.join("worlds")).unwrap();
        fs::write(dir.join("worlds/old.toml"), "old").unwrap();
        fs::write(dir.join("mod.toml"), "keep").unwrap();
        staged(&dir, true);
        assert!(promote(&dir).unwrap());
        assert!(dir.join("worlds/x.toml").is_file() && !dir.join("worlds/old.toml").exists());
        assert!(dir.join("files/WORLDS/X").is_dir());
        assert_eq!(fs::read_to_string(dir.join(stamp::FILE)).unwrap(), "stamp");
        assert!(!dir.join(".staging").exists());
        assert!(dir.join("mod.toml").is_file());
        assert!(!promote(&dir).unwrap(), "nothing is left to promote");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_unfinished_run_is_thrown_away_and_the_content_stays() {
        let dir = temp("cut-short");
        fs::create_dir_all(dir.join("worlds")).unwrap();
        fs::write(dir.join("worlds/old.toml"), "old").unwrap();
        staged(&dir, false);
        assert!(!promote(&dir).unwrap());
        assert!(dir.join("worlds/old.toml").is_file());
        assert!(!dir.join(".staging").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_first_start_writes_the_settings_and_imports_nothing() {
        let dir = temp("first");
        let started = start(&dir, None).unwrap();
        assert!(matches!(started, Start::NotConfigured { created: true, .. }));
        assert!(dir.join("config.toml").is_file());
        assert!(!dir.join("files").exists() && !dir.join("worlds").exists() && !dir.join(STAGING).exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_source_with_no_pack_is_imported_empty_once_and_then_up_to_date() {
        let dir = temp("empty");
        let source = temp("empty-source");
        fs::write(dir.join("config.toml"), format!("source = '{}'\n", source.display())).unwrap();
        let Start::Started { job, live } = start(&dir, None).unwrap() else { panic!("a run should start") };
        assert!(!live);
        let finished = loop {
            if let Some(done) = job.poll() {
                break done;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        let Finished::Done(summary) = finished else { panic!("the run failed") };
        assert!(summary.imported.is_empty() && summary.failed.is_empty());
        assert!(promote(&dir).unwrap());
        assert!(
            matches!(start(&dir, None).unwrap(), Start::UpToDate { worlds: 0 }),
            "nothing changed: nothing converts"
        );
        fs::remove_dir_all(&dir).unwrap();
        fs::remove_dir_all(&source).unwrap();
    }
}
