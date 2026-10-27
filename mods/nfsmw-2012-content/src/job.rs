//! The conversion as a background job, and how its output reaches the mod's folder.
//!
//! The mod folder holds what the game reads (`files/`, `cars/`, the `.import-stamp` of the run that made them). A run
//! never writes there: it builds in `.staging/` and the finished result is *promoted* by renaming. The game has indexed
//! the folder when it started, so a run that finishes while content is already live is promoted at the next start,
//! and a first run, which has nothing live to disturb, is promoted at once and announced to the game.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};

use crate::config::{self, Config};
use crate::import::{self, Report};
use crate::stamp;

const STAGING: &str = ".staging";
/// The folders the game reads, which a promotion replaces together.
const CONTENT: [&str; 3] = ["files", "cars", "manufacturers"];

/// A conversion running on its own thread.
pub struct Job {
    receiver: Receiver<Result<Report>>,
    _thread: JoinHandle<()>,
}

/// What the mod found at start-up.
pub enum Start {
    /// No 2012 install is set. `created` says the settings file was missing and has just been written.
    NotConfigured { config: PathBuf, created: bool },
    /// The content on disk is right for the settings and the installs.
    UpToDate { cars: usize },
    /// A conversion was started; `live` says the folder already holds content the game has indexed.
    Started { job: Job, live: bool },
}

/// What a job did, once it is over.
pub enum Finished {
    Done(Report),
    Failed(String),
}

impl Job {
    /// The result when the job is over, `None` while it runs.
    pub fn poll(&self) -> Option<Finished> {
        match self.receiver.try_recv() {
            Ok(Ok(report)) => Some(Finished::Done(report)),
            Ok(Err(error)) => Some(Finished::Failed(format!("{error:#}"))),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Finished::Failed("the conversion thread stopped".to_owned())),
        }
    }
}

/// What the mod does at load: promote a finished run left by an earlier start, take the settings the host merged from
/// `mod_dir/config.toml` and the game's `[mods.nfsmw-2012-content]` table (`supplied`; without it, read `config.toml`
/// alone), and, unless the content on disk is still right, start converting in the background.
pub fn start(mod_dir: &Path, supplied: Option<&config::Supplied>) -> Result<Start> {
    promote(mod_dir)?;
    let config_path = mod_dir.join(config::FILE_NAME);
    let (config, created) = Config::resolve(supplied, &config_path)?;
    let Some(source) = config.source.clone() else {
        return Ok(Start::NotConfigured { config: config_path, created });
    };
    let stamp = stamp::of(&config, &source);
    if stamp::is_current(mod_dir, &stamp) {
        return Ok(Start::UpToDate { cars: count_cars(mod_dir) });
    }
    let live = mod_dir.join(CONTENT[0]).is_dir();
    let job = spawn(config, source, mod_dir.to_owned(), stamp)?;
    Ok(Start::Started { job, live })
}

fn spawn(config: Config, source: PathBuf, mod_dir: PathBuf, stamp: String) -> Result<Job> {
    let (sender, receiver) = channel();
    let thread = thread::Builder::new()
        .name("nfsmw-2012-content".to_owned())
        .spawn(move || {
            let staging = mod_dir.join(STAGING);
            let _ = fs::remove_dir_all(&staging);
            let result = build(&config, &source, &staging, &stamp);
            if result.is_err() {
                let _ = fs::remove_dir_all(&staging);
            }
            let _ = sender.send(result);
        })
        .context("starting the conversion thread")?;
    Ok(Job { receiver, _thread: thread })
}

fn build(config: &Config, source: &Path, staging: &Path, stamp: &str) -> Result<Report> {
    let report = import::run(config, source, staging, import::default_threads())?;
    // The stamp is written last: a staging folder without one was cut short and is thrown away.
    stamp::save(staging, stamp).context("saving the import stamp")?;
    Ok(report)
}

/// Replace the folders the game reads with the finished run in `.staging/`, if there is one. A staging folder that was
/// not finished is deleted. Returns whether anything was promoted.
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
    fs::remove_dir_all(&staging).with_context(|| format!("removing {}", staging.display()))?;
    Ok(true)
}

fn count_cars(mod_dir: &Path) -> usize {
    let cars = fs::read_dir(mod_dir.join("cars")).into_iter().flatten().flatten();
    cars.filter(|entry| entry.path().extension().is_some_and(|ext| ext == "toml")).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn staged(dir: &Path, finished: bool) {
        fs::create_dir_all(dir.join(".staging/files/CARS/X")).unwrap();
        fs::create_dir_all(dir.join(".staging/cars")).unwrap();
        fs::write(dir.join(".staging/cars/x.toml"), "new").unwrap();
        if finished {
            fs::write(dir.join(".staging").join(stamp::FILE), "stamp").unwrap();
        }
    }

    #[test]
    fn a_finished_run_replaces_the_content_and_keeps_the_rest() {
        let dir = temp("promote");
        fs::create_dir_all(dir.join("cars")).unwrap();
        fs::write(dir.join("cars/old.toml"), "old").unwrap();
        fs::write(dir.join("mod.toml"), "keep").unwrap();
        staged(&dir, true);
        assert!(promote(&dir).unwrap());
        assert!(dir.join("cars/x.toml").is_file() && !dir.join("cars/old.toml").exists());
        assert!(dir.join("files/CARS/X").is_dir());
        assert_eq!(fs::read_to_string(dir.join(stamp::FILE)).unwrap(), "stamp");
        assert!(!dir.join(".staging").exists());
        assert!(dir.join("mod.toml").is_file());
        assert!(!promote(&dir).unwrap(), "nothing is left to promote");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_unfinished_run_is_thrown_away_and_the_content_stays() {
        let dir = temp("cut-short");
        fs::create_dir_all(dir.join("cars")).unwrap();
        fs::write(dir.join("cars/old.toml"), "old").unwrap();
        staged(&dir, false);
        assert!(!promote(&dir).unwrap());
        assert!(dir.join("cars/old.toml").is_file());
        assert!(!dir.join(".staging").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn settings_in_the_main_config_alone_write_no_file_and_convert_nothing_without_a_source() {
        let dir = temp("main-only");
        let text = "role = 'cop'\n".to_owned();
        let supplied = config::Supplied { text, has_file: false, has_main_section: true };
        let start = start(&dir, Some(&supplied)).unwrap();
        assert!(matches!(start, Start::NotConfigured { created: false, .. }));
        assert!(!dir.join("config.toml").exists(), "a template next to a [mods] table would only confuse");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_first_start_writes_the_settings_and_converts_nothing() {
        let dir = temp("first");
        let start = start(&dir, None).unwrap();
        assert!(matches!(start, Start::NotConfigured { created: true, .. }));
        assert!(dir.join("config.toml").is_file());
        assert!(!dir.join("files").exists() && !dir.join("cars").exists() && !dir.join(STAGING).exists());
        assert!(matches!(self::start(&dir, None).unwrap(), Start::NotConfigured { created: false, .. }));
        fs::remove_dir_all(&dir).unwrap();
    }
}
