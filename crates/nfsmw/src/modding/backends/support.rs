//! Test doubles for the backend tests: a scene that keeps what the backends set on it, and a log that tests can read.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, Once};

use anyhow::Result;
use blackbox_gfx::{FrameParams, Instance, RenderBackend};

use super::menu_store::ValueStore;
use crate::hud::MapBlip;
use crate::input::ActionState;
use crate::settings::CameraMode;
use crate::viewer::{CameraReadout, Environment, PlayerCar, Scene, WorldInfo};

/// A scene with a clock it keeps, and a record of what the backends set on it. It draws nothing. The car, the camera and
/// the track are what the scene reports; the moves, the repairs and the settings are what the backends asked for.
#[derive(Default)]
pub(crate) struct Recorder {
    pub clock: Option<Environment>,
    pub held: bool,
    pub blips: Vec<MapBlip>,
    pub car: Option<PlayerCar>,
    pub camera: Option<CameraReadout>,
    pub track: Option<WorldInfo>,
    pub moves: Vec<([f32; 2], f32)>,
    pub repairs: u32,
    pub camera_mode: Option<CameraMode>,
    pub rain: Option<(bool, f32)>,
    pub traffic: Option<(u32, u8)>,
    pub spawns: Vec<([f32; 2], f32)>,
}

impl Recorder {
    /// A scene whose clock stands at `hour`, running at `seconds_per_hour`.
    pub(crate) fn at(hour: f32, seconds_per_hour: f32) -> Self {
        Self { clock: Some(Environment { hour, seconds_per_hour, ..Default::default() }), ..Self::default() }
    }

    /// A scene with no clock, as the menus have.
    pub(crate) fn without_clock() -> Self {
        Self::default()
    }

    /// A driving scene: a car at `position`, facing `heading`, and a camera and a track of its own.
    pub(crate) fn driving(position: [f32; 3], heading: f32) -> Self {
        let car = PlayerCar {
            position,
            velocity: [20.0, 0.0, 0.0],
            heading,
            speed: 20.0,
            rpm: 3500.0,
            gear: 3,
            nos: 0.25,
            damage: 0.0,
        };
        let camera = CameraReadout {
            mode: CameraMode::Chase,
            position: [position[0] - 6.0, position[1], position[2] + 2.0],
            forward: [1.0, 0.0, 0.0],
            fov: 70.0,
        };
        let track = WorldInfo { track: "Bayview".to_owned(), section: Some("Harbor".to_owned()) };
        Self { car: Some(car), camera: Some(camera), track: Some(track), ..Self::default() }
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

    fn player_car(&mut self) -> Option<PlayerCar> {
        self.car
    }

    fn camera_readout(&self) -> Option<CameraReadout> {
        self.camera
    }

    fn world_info(&self) -> Option<WorldInfo> {
        self.track.clone()
    }

    fn move_player(&mut self, at: [f32; 2], heading: f32) -> bool {
        self.moves.push((at, heading));
        self.car.is_some()
    }

    fn repair_player(&mut self) -> bool {
        self.repairs += 1;
        self.car.is_some()
    }

    fn set_camera_mode(&mut self, mode: CameraMode) {
        self.camera_mode = Some(mode);
    }

    fn set_rain(&mut self, always: bool, amount: f32) {
        self.rain = Some((always, amount));
    }

    fn set_traffic_density(&mut self, cars: u32, cop_share: u8) {
        self.traffic = Some((cars, cop_share));
    }

    fn spawn_traffic_at(&mut self, at: [f32; 2], heading: f32) -> bool {
        self.spawns.push((at, heading));
        true
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

/// Row values kept in memory, for the tests that must not write the config file: `(owner, name, value)`, newest last.
#[derive(Default)]
pub(crate) struct MemoryStore {
    pub values: RefCell<Vec<(String, String, toml::Value)>>,
}

impl ValueStore for MemoryStore {
    fn saved(&self, owner: &str, name: &str) -> Option<f32> {
        let values = self.values.borrow();
        let (_, _, value) = values.iter().rev().find(|(known, row, _)| known == owner && row == name)?;
        match value {
            toml::Value::Boolean(on) => Some(f32::from(u8::from(*on))),
            toml::Value::Integer(number) => Some(*number as f32),
            toml::Value::Float(number) => Some(*number as f32),
            _ => None,
        }
    }

    fn save(&self, owner: &str, name: &str, value: toml::Value) -> Result<(), String> {
        self.values.borrow_mut().push((owner.to_owned(), name.to_owned(), value));
        Ok(())
    }
}

/// A shared store: the test keeps one handle and the bridge gets the other.
impl<T: ValueStore + ?Sized> ValueStore for Rc<T> {
    fn saved(&self, owner: &str, name: &str) -> Option<f32> {
        (**self).saved(owner, name)
    }

    fn save(&self, owner: &str, name: &str, value: toml::Value) -> Result<(), String> {
        (**self).save(owner, name, value)
    }
}
