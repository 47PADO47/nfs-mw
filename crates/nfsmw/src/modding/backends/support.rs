//! Test doubles for the backend tests: a scene that keeps what the backends set on it, and a log that tests can read.

use std::sync::{Mutex, Once};

use anyhow::Result;
use blackbox_gfx::{FrameParams, Instance, RenderBackend};

use crate::hud::MapBlip;
use crate::input::ActionState;
use crate::viewer::{Environment, Scene};

/// A scene with a clock it keeps, and a record of what the backends set on it. It draws nothing.
#[derive(Default)]
pub(crate) struct Recorder {
    pub clock: Option<Environment>,
    pub held: bool,
    pub blips: Vec<MapBlip>,
}

impl Recorder {
    /// A scene whose clock stands at `hour`, running at `seconds_per_hour`.
    pub(crate) fn at(hour: f32, seconds_per_hour: f32) -> Self {
        Self { clock: Some(Environment { hour, seconds_per_hour }), ..Self::default() }
    }

    /// A scene with no clock, as the menus have.
    pub(crate) fn without_clock() -> Self {
        Self::default()
    }
}

impl Scene for Recorder {
    fn title(&self) -> String {
        String::new()
    }

    fn init(&mut self, _: &mut dyn RenderBackend) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {}

    fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
        unreachable!("the recorder draws nothing")
    }

    fn environment(&self) -> Option<Environment> {
        self.clock
    }

    fn set_environment(&mut self, value: Environment) {
        self.clock = Some(value);
    }

    fn time_held(&self) -> bool {
        self.held
    }

    fn set_time_held(&mut self, held: bool) {
        self.held = held;
    }

    fn set_mod_blips(&mut self, blips: &[MapBlip]) {
        self.blips = blips.to_vec();
    }
}

static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Sends the log to [`LINES`]. Only the first logger installed in a test process counts.
struct Capture;

impl log::Log for Capture {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        LINES.lock().unwrap_or_else(|e| e.into_inner()).push(record.args().to_string());
    }

    fn flush(&self) {}
}

/// Every log line so far, oldest first. The first call installs the capture.
pub(crate) fn captured_log() -> Vec<String> {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        if log::set_boxed_logger(Box::new(Capture)).is_ok() {
            log::set_max_level(log::LevelFilter::Info);
        }
    });
    LINES.lock().unwrap_or_else(|e| e.into_inner()).clone()
}
