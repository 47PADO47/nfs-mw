//! Player preferences and explicit, session-only testing overrides for F5.
pub mod storage;
pub mod view;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraPreferences {
    pub auto_center: bool,
    pub center_delay: f32,
    pub center_speed: f32,
    pub distance: f32,
    pub height: f32,
    pub fov: f32,
    pub speed_zoom: bool,
    pub preserve_framing: bool,
    pub center_key: String,
    pub hide_ui_key: String,
}
impl Default for CameraPreferences {
    fn default() -> Self {
        Self {
            auto_center: true,
            center_delay: 1.2,
            center_speed: 3.0,
            distance: 5.6,
            height: 1.9,
            fov: 60.0,
            speed_zoom: true,
            preserve_framing: true,
            center_key: "Home".into(),
            hide_ui_key: "F4".into(),
        }
    }
}
impl CameraPreferences {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.center_delay.is_finite() && (0.0..=30.0).contains(&self.center_delay),
            "Invalid center delay"
        );
        anyhow::ensure!(
            self.center_speed.is_finite() && (0.1..=20.0).contains(&self.center_speed),
            "Invalid center speed"
        );
        anyhow::ensure!(self.distance.is_finite() && (1.0..=30.0).contains(&self.distance), "Invalid camera distance");
        anyhow::ensure!(self.height.is_finite() && (0.0..=10.0).contains(&self.height), "Invalid camera height");
        anyhow::ensure!(self.fov.is_finite() && (20.0..=120.0).contains(&self.fov), "Invalid camera FOV");
        anyhow::ensure!(["Home", "Insert", "End"].contains(&self.center_key.as_str()), "Invalid center shortcut");
        anyhow::ensure!(["F4", "F3", "F2"].contains(&self.hide_ui_key.as_str()), "Invalid screenshot shortcut");
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Overrides {
    pub traffic_off: bool,
    pub police_off: bool,
    pub unlimited_nos: bool,
    pub unlimited_breaker: bool,
    pub paused: bool,
    pub lock_heat: bool,
    pub heat: u8,
}
impl Default for Overrides {
    fn default() -> Self {
        Self {
            traffic_off: false,
            police_off: false,
            unlimited_nos: false,
            unlimited_breaker: false,
            paused: false,
            lock_heat: false,
            heat: 1,
        }
    }
}
impl Overrides {
    pub fn active(&self) -> bool {
        self != &Self::default()
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!((1..=10).contains(&self.heat), "Heat must be 1 through 10");
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub position: [f32; 2],
    pub heading: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub relative: bool,
    pub eye: [f32; 3],
    pub forward: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
}
#[derive(Default)]
pub struct State {
    pub persistence_error: Option<String>,
    pub local_allowed: bool,
    pub teleport_heading: Option<f32>,
    pub free_roll: f32,
    pub restore_environment: Option<crate::viewer::Environment>,
    pub environment: Option<crate::viewer::Environment>,
    pub applied_heat: Option<u8>,
    pub camera: CameraPreferences,
    pub testing: Overrides,
    pub locations: Vec<Location>,
    pub bookmarks: Vec<Bookmark>,
    pub previous_location: Option<Location>,
    pub selected_bookmark: Option<Bookmark>,
    pub freecam: bool,
    pub free_fov: Option<f32>,
    pub hide_ui: bool,
    pub tab: usize,
    pub name: String,
    pub relative: bool,
    pub coordinates: [f32; 2],
    pub message: Option<String>,
    pub profiles: crate::preset_store::Picker,
}
#[derive(Clone)]
pub enum Action {
    Environment(crate::viewer::Environment),
    Center,
    Freecam,
    EndPursuit,
    StartPursuit,
    SetHeat,
    Refill,
    Recover,
    Repair,
    Teleport([f32; 2], Option<f32>),
    Marker,
    Return,
    SaveLocation,
    DeleteLocation(usize),
    SaveBookmark,
    LoadBookmark(usize),
    DeleteBookmark(usize),
    Reload,
    Clear,
}
