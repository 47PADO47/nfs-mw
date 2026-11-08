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
use crate::{lazy, stamp};

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
    /// The content on disk is right for the settings and the installs. `trees` is the setting of the same name.
    UpToDate { cars: usize, trees: bool },
    /// A conversion was started; `live` says the folder already holds content the game has indexed. `trees` is the setting of
    /// the same name.
    Started { job: Job, live: bool, trees: bool },
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
/// alone), and, unless the content on disk is still right, start converting in the background. The conversion writes every
/// file; [`start_with`] can have it write lazy files.
pub fn start(mod_dir: &Path, supplied: Option<&config::Supplied>) -> Result<Start> {
    start_with(mod_dir, supplied, |_| false)
}

/// [`start`], where `streaming` says, for the settings of this start, whether the conversion writes lazy files instead of the
/// cars' heavy files (`lazy.rs`). It is asked once, when a 2012 install is set, and its answer is part of the import stamp, so
/// a run that wrote files is not taken for one that wrote stubs.
pub fn start_with(
    mod_dir: &Path,
    supplied: Option<&config::Supplied>,
    streaming: impl FnOnce(&Config) -> bool,
) -> Result<Start> {
    promote(mod_dir)?;
    let config_path = mod_dir.join(config::FILE_NAME);
    let (config, created) = Config::resolve(supplied, &config_path)?;
    // The soundtrack is a playlist of the install's own files, kept in line with the settings at every start whatever the
    // conversion does (`soundtrack.rs`).
    crate::soundtrack::sync(mod_dir, config.source.as_deref(), config.music);
    let Some(source) = config.source.clone() else {
        return Ok(Start::NotConfigured { config: config_path, created });
    };
    let config = Config { lazy: streaming(&config), ..config };
    let stamp = stamp::of(&config, &source);
    if stamp::is_current(mod_dir, &stamp) {
        return Ok(Start::UpToDate { cars: count_cars(mod_dir), trees: config.trees });
    }
    let live = mod_dir.join(CONTENT[0]).is_dir();
    let trees = config.trees;
    let job = spawn(config, source, mod_dir.to_owned(), stamp)?;
    Ok(Start::Started { job, live, trees })
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
        remove_staging(&staging);
        return Ok(false);
    }
    stamp::clear(mod_dir);
    // The recipe of the lazy files belongs to the content it was written with: the old one goes, the new one (if the run was
    // lazy) takes its place.
    let recipe = mod_dir.join(lazy::RECIPE_FILE);
    if recipe.exists() {
        fs::remove_file(&recipe).with_context(|| format!("removing {}", recipe.display()))?;
    }
    let staged_recipe = staging.join(lazy::RECIPE_FILE);
    if staged_recipe.exists() {
        fs::rename(&staged_recipe, &recipe).context("moving the recipe of the lazy files into place")?;
    }
    for part in CONTENT {
        let (live, staged) = (mod_dir.join(part), staging.join(part));
        if staged.exists() {
            crate::swap::replace_dir(&live, &staged).with_context(|| format!("replacing {}", live.display()))?;
            continue;
        }
        if live.exists() {
            crate::swap::replace_dir(&live, &staged).with_context(|| format!("clearing {}", live.display()))?;
        }
    }
    fs::rename(staging.join(stamp::FILE), mod_dir.join(stamp::FILE)).context("moving the import stamp into place")?;
    remove_staging(&staging);
    Ok(true)
}

/// Delete the staging folder. Best effort: Windows refuses to delete a folder another process has open (a file indexer or a virus
/// scanner looking at what was just written), and an empty leftover is harmless (the next start finds nothing to promote and
/// tries again), whereas failing here would throw away a finished promotion.
fn remove_staging(staging: &Path) {
    if let Err(error) = fs::remove_dir_all(staging) {
        log::debug!("the staging folder {} stays for now: {error}", staging.display());
    }
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
    fn a_lazy_run_brings_its_recipe_and_a_later_run_without_one_takes_it_away() {
        let dir = temp("recipe");
        fs::write(dir.join(lazy::RECIPE_FILE), "old recipe").unwrap();
        staged(&dir, true);
        fs::write(dir.join(".staging").join(lazy::RECIPE_FILE), "new recipe").unwrap();
        assert!(promote(&dir).unwrap());
        assert_eq!(fs::read_to_string(dir.join(lazy::RECIPE_FILE)).unwrap(), "new recipe");
        staged(&dir, true);
        assert!(promote(&dir).unwrap());
        assert!(!dir.join(lazy::RECIPE_FILE).exists(), "content that writes every file has no recipe");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_decision_to_stream_is_asked_with_the_settings_and_changes_what_up_to_date_means() {
        let dir = temp("streaming");
        let source = temp("streaming-source");
        let supplied = config::Supplied {
            text: format!("source = '{}'\n", source.to_string_lossy().replace('\\', "/")),
            has_file: false,
            has_main_section: true,
        };
        // Pretend a run for the settings without streaming finished: the stamp is the eager one.
        let eager = Config::parse(&supplied.text).unwrap();
        stamp::save(&dir, &stamp::of(&eager, &source)).unwrap();
        assert!(matches!(start_with(&dir, Some(&supplied), |_| false).unwrap(), Start::UpToDate { .. }));
        let asked = std::cell::Cell::new(false);
        let started = start_with(&dir, Some(&supplied), |config| {
            asked.set(config.source.is_some());
            true
        });
        assert!(asked.get(), "the decision sees the settings, install included");
        assert!(!matches!(started, Ok(Start::UpToDate { .. })), "stubs are not what the eager run wrote");
        fs::remove_dir_all(&dir).unwrap();
        fs::remove_dir_all(&source).unwrap();
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
