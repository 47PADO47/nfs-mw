//! The import as a background job, and how its output reaches the mod's folder.
//!
//! The mod folder holds what the game reads (`files/`, `cars/`, the `.import-stamp` of the run that made them). A run never
//! writes there: it builds in `.staging/` and the finished result is *promoted* by renaming. The game has indexed the folder
//! when it started, so a run that finishes while content is already live is promoted at the next start, and a first run,
//! which has nothing live to disturb, is promoted at once and announced to the game. (The same scheme as
//! `mods/nfsmw-2012-content/src/job.rs`.)

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};

use crate::batch::{self, Report};
use crate::config::{self, Config};
use crate::stamp;

const STAGING: &str = ".staging";
/// The rider's riding style: 1 is the plain one (`ride6-rider-import --style`).
const RIDING_STYLE: u8 = 1;
/// The two files of the rider's export folder that make his ragdoll.
const RAGDOLL_ASSET: &str = "RiderGP_PhysicsAsset.json";
const RAGDOLL_BLUEPRINT: &str = "RiderBP.json";
/// The folders the game reads, which a promotion replaces together.
const CONTENT: [&str; 2] = ["files", "cars"];

/// An import running on its own thread.
pub struct Job {
    receiver: Receiver<Result<Report>>,
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// What the mod found at start-up.
pub enum Start {
    /// No `source` is set. `created` says the settings file was missing and has just been written.
    NotConfigured { config: PathBuf, created: bool },
    /// The content on disk is right for the settings and the inputs.
    UpToDate { cars: usize },
    /// An import was started; `live` says the folder already holds content the game has indexed.
    Started { job: Job, live: bool },
}

/// What a job did, once it is over.
pub enum Finished {
    Done(Report),
    Failed(String),
}

/// The game unloads the mod's library when it closes, and a thread still running in it would then run unloaded code: the
/// job is stopped before its next bike and waited for.
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
            Ok(Ok(report)) => Some(Finished::Done(report)),
            Ok(Err(error)) => Some(Finished::Failed(format!("{error:#}"))),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Finished::Failed("the import thread stopped".to_owned())),
        }
    }
}

/// The folders a run reads, from the settings.
struct Inputs {
    source: PathBuf,
    physics: PathBuf,
    tyre_tables: Option<PathBuf>,
    sound: Option<PathBuf>,
    rider: Option<PathBuf>,
}

impl Inputs {
    fn of(config: &Config, source: PathBuf) -> Self {
        let physics = config.physics_dir(&source);
        let tyre_tables = config.tyre_dir(&physics);
        let rider = config.rider_dir(&source);
        Self { source, physics, tyre_tables, sound: config.sound.clone(), rider }
    }
}

/// What the mod does at load: promote a finished run left by an earlier start, take the settings the host merged from
/// `mod_dir/config.toml` and the game's `[mods.ride6-bikes]` table (`supplied`; without it, read `config.toml` alone), and,
/// unless the content on disk is still right, start importing in the background.
pub fn start(mod_dir: &Path, supplied: Option<&config::Supplied>) -> Result<Start> {
    promote(mod_dir)?;
    let config_path = mod_dir.join(config::FILE_NAME);
    let (config, created) = Config::resolve(supplied, &config_path)?;
    let Some(source) = config.source.clone() else {
        return Ok(Start::NotConfigured { config: config_path, created });
    };
    let inputs = Inputs::of(&config, source);
    let (found, _) = batch::discover(&inputs.source)?;
    let dirs: Vec<PathBuf> = found.into_iter().map(|bike| bike.dir).collect();
    let stamp = stamp::of(&config, &dirs, &inputs.physics, inputs.sound.as_deref(), inputs.rider.as_deref());
    if stamp::is_current(mod_dir, &stamp) {
        return Ok(Start::UpToDate { cars: count_cars(mod_dir) });
    }
    let live = mod_dir.join(CONTENT[0]).is_dir();
    let job = spawn(config.max_texture, inputs, mod_dir.to_owned(), stamp)?;
    Ok(Start::Started { job, live })
}

fn spawn(max_texture: u32, inputs: Inputs, mod_dir: PathBuf, stamp: String) -> Result<Job> {
    let (sender, receiver) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&cancel);
    let thread = thread::Builder::new()
        .name(config::MOD_ID.to_owned())
        .stack_size(batch::STACK_BYTES)
        .spawn(move || {
            let staging = mod_dir.join(STAGING);
            let _ = fs::remove_dir_all(&staging);
            let result = build(max_texture, &inputs, &staging, &stamp, &stop);
            if result.is_err() {
                let _ = fs::remove_dir_all(&staging);
            }
            let _ = sender.send(result);
        })
        .context("starting the import thread")?;
    Ok(Job { receiver, cancel, thread: Some(thread) })
}

/// The rider's pack from his export folder: the riding pack, and the fall's `ragdoll.json` when the folder also holds the two
/// files it is made from (`RiderGP_PhysicsAsset.json`, `RiderBP.json`); without them the rider stays on a bike that falls.
fn rider_files(export: &Path, files: &Path) -> Result<()> {
    crate::rider::import(export, files, RIDING_STYLE)?;
    let (asset, blueprint) = (export.join(RAGDOLL_ASSET), export.join(RAGDOLL_BLUEPRINT));
    if !asset.is_file() || !blueprint.is_file() {
        return Ok(());
    }
    crate::rider_ragdoll::import(&asset, &blueprint, files)?;
    Ok(())
}

fn build(max_texture: u32, inputs: &Inputs, staging: &Path, stamp: &str, cancel: &AtomicBool) -> Result<Report> {
    let input = batch::Input {
        source: &inputs.source,
        physics: &inputs.physics,
        tyre_tables: inputs.tyre_tables.as_deref(),
        sound: inputs.sound.as_deref(),
        max_texture,
        threads: batch::default_threads(),
        cancel,
    };
    fs::create_dir_all(staging).with_context(|| format!("creating {}", staging.display()))?;
    let mut report = batch::run(&input, staging)?;
    if let Some(rider) = &inputs.rider {
        let files = staging.join("files").join("RIDER");
        report.rider_failed = rider_files(rider, &files).err().map(|error| format!("{error:#}"));
    }
    // The stamp is written last: a staging folder without one was cut short and is thrown away.
    stamp::save(staging, stamp).context("saving the import stamp")?;
    Ok(report)
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
        let dir = std::env::temp_dir().join(format!("ride6-bikes-job-{name}-{}", std::process::id()));
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
    fn a_first_start_writes_the_settings_and_imports_nothing() {
        let dir = temp("first");
        let start = start(&dir, None).unwrap();
        assert!(matches!(start, Start::NotConfigured { created: true, .. }));
        assert!(dir.join("config.toml").is_file());
        assert!(!dir.join("files").exists() && !dir.join("cars").exists() && !dir.join(STAGING).exists());
        assert!(matches!(self::start(&dir, None).unwrap(), Start::NotConfigured { created: false, .. }));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_source_with_no_bike_is_imported_empty_once_and_then_up_to_date() {
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
        let Finished::Done(report) = finished else { panic!("the run failed") };
        assert!(report.imported.is_empty() && report.skipped.is_empty());
        assert!(promote(&dir).unwrap());
        assert!(matches!(start(&dir, None).unwrap(), Start::UpToDate { cars: 0 }), "nothing changed: nothing converts");
        fs::remove_dir_all(&dir).unwrap();
        fs::remove_dir_all(&source).unwrap();
    }
}
