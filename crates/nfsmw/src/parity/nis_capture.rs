//! Off-screen frames of the NIS scenes a run plays, for looking at them (`docs/testing.md`, "NIS frame capture"). Off
//! unless `NFSMW_NIS_CAPTURE=<folder>` names a folder.
//!
//! While a captured scene plays, its clock steps one scene frame (1/30 s) per drawn frame instead of the frame's real
//! time, and holds while a frame is being read back, so the frames and the record do not depend on how fast the machine
//! draws. Every `NFSMW_NIS_CAPTURE_EVERY` scene frames (default 15) the world asks for a frame; the render bridge draws
//! it off-screen at `NFSMW_NIS_CAPTURE_SIZE` (default 640x360) and writes `<scene>-<frame>.png`. Every scene frame adds
//! a line to `frames.jsonl`: each car channel's pose from the scene data and the pose the car is drawn at, and the
//! camera from the track and the lens the frame used (`scenes/world/nis/capture.rs`). Story movies are skipped while
//! the capture is on. `NFSMW_NIS_CAPTURE_SCENES=IntroNisDD,IntroNis05` limits it to those scenes, and
//! `NFSMW_NIS_CAPTURE_EXIT=1` ends the process once each of them has played.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use blackbox_gfx::CaptureId;
use serde_json::Value;

pub const DIR_ENV: &str = "NFSMW_NIS_CAPTURE";
pub const EVERY_ENV: &str = "NFSMW_NIS_CAPTURE_EVERY";
pub const SIZE_ENV: &str = "NFSMW_NIS_CAPTURE_SIZE";
pub const SCENES_ENV: &str = "NFSMW_NIS_CAPTURE_SCENES";
pub const EXIT_ENV: &str = "NFSMW_NIS_CAPTURE_EXIT";

/// What the run captures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub dir: PathBuf,
    /// A frame every this many scene frames (at least 1).
    pub every: u32,
    pub size: [u32; 2],
    /// The scenes to capture, lower case; empty is every scene.
    pub scenes: Vec<String>,
    pub exit_when_done: bool,
}

impl Config {
    /// The capture the environment asks for through `get`, or `None` when no folder is named.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let dir = get(DIR_ENV).filter(|value| !value.trim().is_empty()).map(PathBuf::from)?;
        let every = get(EVERY_ENV).and_then(|value| value.trim().parse().ok()).unwrap_or(15u32).max(1);
        let size = get(SIZE_ENV).and_then(|value| parse_size(&value)).unwrap_or([640, 360]);
        let scenes = get(SCENES_ENV)
            .map(|value| value.split(',').map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default();
        let exit_when_done = get(EXIT_ENV).is_some_and(|value| matches!(value.trim(), "1" | "true" | "yes"));
        Some(Self { dir, every, size, scenes, exit_when_done })
    }

    /// Whether the scene `name` is captured.
    pub fn wants(&self, name: &str) -> bool {
        self.scenes.is_empty() || self.scenes.iter().any(|scene| scene.eq_ignore_ascii_case(name))
    }
}

/// `WIDTHxHEIGHT`, both at least 16.
fn parse_size(value: &str) -> Option<[u32; 2]> {
    let (w, h) = value.trim().split_once(['x', 'X'])?;
    let size = [w.trim().parse().ok()?, h.trim().parse().ok()?];
    size.iter().all(|&side| side >= 16).then_some(size)
}

struct State {
    config: Config,
    record: Option<File>,
    /// A frame the world asked for that the render bridge has not started.
    requested: Option<PathBuf>,
    /// A frame the renderer is drawing.
    in_flight: Option<(CaptureId, PathBuf)>,
    /// The wanted scenes that have played to their end.
    done: Vec<String>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static ON: AtomicBool = AtomicBool::new(false);

/// Turns the capture on when the environment asks for it. Called once, when the command starts.
pub fn init_from_env() {
    let Some(config) = Config::from_env(|name| std::env::var(name).ok()) else { return };
    if let Err(e) = std::fs::create_dir_all(&config.dir) {
        return log::error!("NIS capture: cannot create {}: {e}", config.dir.display());
    }
    let record = File::create(config.dir.join("frames.jsonl"))
        .inspect_err(|e| log::error!("NIS capture: cannot create frames.jsonl: {e}"))
        .ok();
    log::info!("NIS capture: every {} scene frames into {}", config.every, config.dir.display());
    if let Ok(mut state) = STATE.lock() {
        *state = Some(State { config, record, requested: None, in_flight: None, done: Vec::new() });
    }
    ON.store(true, Ordering::Relaxed);
}

/// Whether the capture is on at all.
pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

fn with<T>(f: impl FnOnce(&mut State) -> T) -> Option<T> {
    if !enabled() {
        return None;
    }
    STATE.lock().ok()?.as_mut().map(f)
}

/// Whether the scene `name` is captured.
pub fn wants(name: &str) -> bool {
    with(|state| state.config.wants(name)).unwrap_or(false)
}

/// The scene frames between two captured frames.
pub fn every() -> u32 {
    with(|state| state.config.every).unwrap_or(1)
}

/// Whether a frame is asked for or being drawn: the captured scene's clock waits for it.
pub fn busy() -> bool {
    with(|state| state.requested.is_some() || state.in_flight.is_some()).unwrap_or(false)
}

/// Asks for the next drawn frame as `<scene>-<frame>.png`.
pub fn request(scene: &str, frame: u32) {
    with(|state| state.requested = Some(state.config.dir.join(format!("{scene}-{frame:05}.png"))));
}

/// Adds one line to `frames.jsonl`. A line that cannot be written ends the record.
pub fn record(line: &Value) {
    with(|state| {
        let Some(out) = state.record.as_mut() else { return };
        if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
            log::error!("NIS capture: frames.jsonl cannot be written; the record stops");
            state.record = None;
        }
    });
}

/// The scene `name` ended. When every wanted scene has played and the run asked for it, the process ends.
pub fn scene_ended(name: &str) {
    let finished = with(|state| {
        if !state.config.wants(name) || state.config.scenes.is_empty() {
            return false;
        }
        let name = name.to_ascii_lowercase();
        if !state.done.contains(&name) {
            state.done.push(name);
        }
        state.config.exit_when_done && state.config.scenes.iter().all(|scene| state.done.contains(scene))
    });
    if finished == Some(true) {
        log::info!("NIS capture: every scene asked for has played; the process ends");
        super::race_run::request_exit(Ok(()));
    }
}

/// The frame the world asked for, to start drawing now at the capture size. The bridge hands it back with
/// [`started`] once the renderer has it.
pub fn take_request() -> Option<(PathBuf, [u32; 2])> {
    with(|state| state.requested.take().map(|path| (path, state.config.size))).flatten()
}

/// The renderer draws the frame for `path` under `id`.
pub fn started(id: CaptureId, path: PathBuf) {
    with(|state| state.in_flight = Some((id, path)));
}

/// The frame being drawn, if one is.
pub fn in_flight() -> Option<(CaptureId, PathBuf)> {
    with(|state| state.in_flight.clone()).flatten()
}

/// The frame being drawn is written (or failed): the scene's clock goes on.
pub fn finished() {
    with(|state| state.in_flight = None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_environment_names_the_folder_the_step_the_size_and_the_scenes() {
        let env = |name: &str| match name {
            DIR_ENV => Some("out/nis".to_owned()),
            EVERY_ENV => Some("10".to_owned()),
            SIZE_ENV => Some("480x270".to_owned()),
            SCENES_ENV => Some("IntroNisDD, IntroNis05".to_owned()),
            EXIT_ENV => Some("1".to_owned()),
            _ => None,
        };
        let config = Config::from_env(env).unwrap();
        assert_eq!(config.every, 10);
        assert_eq!(config.size, [480, 270]);
        assert!(config.wants("introNISdd") && config.wants("IntroNis05") && !config.wants("IntroNis10"));
        assert!(config.exit_when_done);
        assert_eq!(Config::from_env(|_| None), None);
        let defaults = Config::from_env(|name| (name == DIR_ENV).then(|| "x".to_owned())).unwrap();
        assert_eq!((defaults.every, defaults.size, defaults.wants("Any")), (15, [640, 360], true));
        assert_eq!(parse_size("8x8"), None);
    }
}
