//! The unattended race run: start in a race, let the opponents' driver take the player's car, record the race and end
//! the process when it is over. The command line sets the options once (`cli/race_args.rs`); the first world of the run
//! takes them (`scenes/world/race/unattended.rs`). Nothing here is saved, and with no option given nothing changes.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// The variable that names the race trace file, as `--race-trace` does.
pub const RACE_TRACE_ENV: &str = "NFSMW_RACE_TRACE";
/// The skill of the autopilot when `--autopilot-skill` does not say: the opponents' scale, 0 to 1, with no catch-up.
pub const AUTOPILOT_SKILL: f32 = 1.0;

/// The race the run starts in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaceRequest {
    /// What the `race start` console command takes: a career event id (`16.1.1`) or a race's collection name.
    pub event: String,
    /// The number of opponents; `None` keeps the event's own.
    pub opponents: Option<usize>,
    /// The number of laps; `None` keeps the event's own.
    pub laps: Option<u32>,
}

/// The opponents' driver in the player's car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Autopilot {
    /// Seconds of race time after the green light at which the driver takes the car; 0 takes it at the green light.
    pub after_secs: f32,
    /// The driver's skill, 0 to 1.
    pub skill: f32,
}

/// What an unattended run was started with.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RaceRunOptions {
    pub race: Option<RaceRequest>,
    pub autopilot: Option<Autopilot>,
    /// The file the race trace is written to (`race_trace.rs`).
    pub trace: Option<PathBuf>,
    /// End the process when the race is over.
    pub exit_after_race: bool,
}

impl RaceRunOptions {
    /// The trace file: the command line's over the environment's, read through `get`. An empty value is no file.
    pub fn with_trace(mut self, flag: Option<PathBuf>, get: impl Fn(&str) -> Option<String>) -> Self {
        let named = get(RACE_TRACE_ENV).filter(|value| !value.trim().is_empty()).map(PathBuf::from);
        self.trace = flag.or(named);
        self
    }
}

static OPTIONS: OnceLock<RaceRunOptions> = OnceLock::new();
static RACE_TAKEN: AtomicBool = AtomicBool::new(false);
static EXIT: Mutex<Option<Result<(), String>>> = Mutex::new(None);

/// Sets the run's options. Called once, when the command starts; a later call changes nothing.
pub fn init(options: RaceRunOptions) {
    if options != RaceRunOptions::default() {
        log::info!("unattended race run: {options:?}");
    }
    let _ = OPTIONS.set(options);
}

/// The options a world takes: the run's, with the race only for the first world that asks. A later world of the same
/// run (the game went back to the menus and on again) starts no race by itself.
pub fn take() -> RaceRunOptions {
    let Some(options) = OPTIONS.get() else { return RaceRunOptions::default() };
    let first = !RACE_TAKEN.swap(true, Ordering::Relaxed);
    RaceRunOptions { race: options.race.clone().filter(|_| first), ..options.clone() }
}

/// Asks the app to end the process: cleanly, or with `Err(why)` as a failure (the race could not be started).
/// The first request stands.
pub fn request_exit(outcome: Result<(), String>) {
    if let Ok(mut exit) = EXIT.lock() {
        exit.get_or_insert(outcome);
    }
}

/// The exit a world asked for, taken once by the app's end of frame (`app/pacing.rs`).
pub fn take_exit() -> Option<Result<(), String>> {
    EXIT.lock().ok().and_then(|mut exit| exit.take())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| (*value).to_owned())
    }

    #[test]
    fn nothing_given_is_no_run() {
        let options = RaceRunOptions::default().with_trace(None, env(&[]));
        assert_eq!(options, RaceRunOptions::default());
        assert!(options.race.is_none() && options.autopilot.is_none() && !options.exit_after_race);
    }

    #[test]
    fn the_trace_file_comes_from_the_flag_then_the_environment() {
        let named = env(&[(RACE_TRACE_ENV, "from-env.jsonl")]);
        let options = RaceRunOptions::default().with_trace(None, &named);
        assert_eq!(options.trace, Some(PathBuf::from("from-env.jsonl")));
        let options = RaceRunOptions::default().with_trace(Some("flag.jsonl".into()), &named);
        assert_eq!(options.trace, Some(PathBuf::from("flag.jsonl")), "the command line wins");
        let options = RaceRunOptions::default().with_trace(None, env(&[(RACE_TRACE_ENV, "  ")]));
        assert_eq!(options.trace, None, "an empty variable names no file");
    }
}
