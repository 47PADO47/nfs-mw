//! The header every interface table starts with, and the ids and versions of the generic interfaces (ADR 0005
//! sections 1 and 8).
//!
//! An interface table is a `repr(C)` struct: `head: BbIface` first, then `ctx`, then the function pointers. `ctx` is
//! the host's own pointer for that table, and every function takes it as its first argument. A function pointer is
//! `Option<fn>`; `None`, or a field the table's `head.size` does not cover, means "not supported".

mod audio;
mod camera;
mod commands;
mod config;
mod content;
mod engine;
mod hud;
mod i18n;
mod input;
mod map;
mod menus;
mod settings;
mod time;
mod traffic;
mod ui;
mod vehicles;
mod world;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod traffic_tests;

pub use audio::{BbAudio, BbAudioPlayFn};
pub use camera::{
    BB_CAMERA_BUMPER, BB_CAMERA_CHASE, BB_CAMERA_CLOSE, BB_CAMERA_DRIFT, BB_CAMERA_DYNAMIC_FOV, BB_CAMERA_EXTRA,
    BB_CAMERA_FAR, BB_CAMERA_HOOD, BB_CAMERA_INPUT_RESET, BB_CAMERA_POSE_HIDE_CAR, BB_CAMERA_PURSUIT,
    BB_CAMERA_SUPER_FAR, BbCamera, BbCameraAvailableFn, BbCameraGetFn, BbCameraInput, BbCameraPose,
    BbCameraRegisterViewFn, BbCameraSelectViewFn, BbCameraSetModeFn, BbCameraState, BbCameraUnregisterViewFn,
    BbCameraUpdateFn, BbCameraView,
};
pub use commands::{BbCommandCallbackFn, BbCommands, BbCommandsRegisterFn, BbCommandsRunFn, BbCommandsUnregisterFn};
pub use config::{BB_CONFIG_FILE, BB_CONFIG_MAIN, BbConfig, BbConfigReadFn, BbConfigSourcesFn};
pub use content::{BbContent, BbContentRescanFn};
pub use engine::{
    BB_ENGINE_OFF, BB_ENGINE_RUNNING, BB_ENGINE_STARTING, BbEngine, BbEngineCarFn, BbEngineStartFn, BbEngineState,
    BbEngineStateFn, BbEngineStopFn,
};
pub use hud::{BB_HUD_BIG, BB_HUD_INFO, BB_HUD_WARNING, BbHud, BbHudHideFn, BbHudMessageFn};
pub use i18n::{BB_I18N_INT, BB_I18N_TEXT, BbI18n, BbI18nArg, BbI18nGenerationFn, BbI18nLocaleFn, BbI18nTextFn};
pub use input::{BbHotkeyFn, BbInput, BbInputRegisterFn, BbInputUnregisterFn};
pub use map::{BbBlip, BbMap, BbMapAddFn, BbMapRemoveFn, BbMapUpdateFn, BbRgba};
pub use menus::{
    BB_MENU_BUTTON, BB_MENU_LIST, BB_MENU_SLIDER, BB_MENU_TOGGLE, BbMenuCallbackFn, BbMenuRow, BbMenus, BbMenusGetFn,
    BbMenusRegisterFn, BbMenusSetFn, BbMenusUnregisterFn,
};
pub use settings::{
    BB_SETTING_LIST, BB_SETTING_MENU_GAMEPLAY, BB_SETTING_TOGGLE, BbSettingCallbackFn, BbSettingOption, BbSettingValue,
    BbSettings, BbSettingsAddOptionFn, BbSettingsGetFn, BbSettingsRemoveOptionFn,
};
pub use time::{BbTime, BbTimeFreezeFn, BbTimeGetFn, BbTimeOfDay, BbTimeSetDayLengthFn, BbTimeSetHourFn};
pub use traffic::{BB_TRAFFIC_FIELD, BbTraffic, BbTrafficListFn, BbTrafficSpawnFn};
pub use ui::{
    BB_UI_IMAGE, BB_UI_PANEL, BB_UI_TEXT, BbUi, BbUiPrim, BbUiRegisterThemeFn, BbUiSetSlotFn, BbUiSetTokensFn,
    BbUiTokens, BbUiUnregisterThemeFn,
};
pub use vehicles::{
    BbVec3, BbVehicleAddNosFn, BbVehicleRepairFn, BbVehicleSetDriftAssistFn, BbVehicleSpawnTrafficFn, BbVehicleState,
    BbVehicleStateFn, BbVehicleTeleportFn, BbVehicles,
};
pub use world::{BbWorld, BbWorldSetRainFn, BbWorldSetTrafficFn, BbWorldTextFn};

/// The header of an interface table. Its `size` says how many bytes the host filled in, and `major` and `minor` are
/// the interface's own version (for example `bb.map 1.2`), separate from the ABI version.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbIface {
    /// Bytes the table holds.
    pub size: u32,
    /// Interface major version.
    pub major: u16,
    /// Interface minor version.
    pub minor: u16,
}

/// Id of the command interface: run console lines and register mod commands.
pub const IFACE_COMMANDS: &str = "bb.commands";
/// The version the host implements for [`IFACE_COMMANDS`], as `(major, minor)`.
pub const IFACE_COMMANDS_VERSION: (u16, u16) = (1, 0);

/// Id of the input interface: register hotkeys.
pub const IFACE_INPUT: &str = "bb.input";
/// The version the host implements for [`IFACE_INPUT`], as `(major, minor)`.
pub const IFACE_INPUT_VERSION: (u16, u16) = (1, 0);

/// Id of the content interface: tell the host that the files a mod keeps in its folder have changed.
pub const IFACE_CONTENT: &str = "bb.content";
/// The version the host implements for [`IFACE_CONTENT`], as `(major, minor)`.
pub const IFACE_CONTENT_VERSION: (u16, u16) = (1, 0);
/// Id of the time-of-day interface: read and set the world clock.
pub const IFACE_TIME: &str = "bb.time";
/// The version the host implements for [`IFACE_TIME`], as `(major, minor)`.
pub const IFACE_TIME_VERSION: (u16, u16) = (1, 0);

/// Id of the map interface: add, move and remove blips.
pub const IFACE_MAP: &str = "bb.map";
/// The version the host implements for [`IFACE_MAP`], as `(major, minor)`.
pub const IFACE_MAP_VERSION: (u16, u16) = (1, 0);

/// Id of the menus interface: rows on the Mods page of the options.
pub const IFACE_MENUS: &str = "bb.menus";
/// The version the host implements for [`IFACE_MENUS`], as `(major, minor)`.
pub const IFACE_MENUS_VERSION: (u16, u16) = (1, 0);

/// Id of the settings interface: options (and values of options) a mod adds to the game's settings screens.
pub const IFACE_SETTINGS: &str = "bb.settings";
/// The version the host implements for [`IFACE_SETTINGS`], as `(major, minor)`.
pub const IFACE_SETTINGS_VERSION: (u16, u16) = (1, 0);

/// Id of the vehicles interface: read and move the player's car.
pub const IFACE_VEHICLES: &str = "bb.vehicles";
/// The version the host implements for [`IFACE_VEHICLES`], as `(major, minor)`.
pub const IFACE_VEHICLES_VERSION: (u16, u16) = (1, 2);

/// Id of the HUD interface: a centred message and hiding it.
pub const IFACE_HUD: &str = "bb.hud";
/// The version the host implements for [`IFACE_HUD`], as `(major, minor)`.
pub const IFACE_HUD_VERSION: (u16, u16) = (1, 0);

/// Id of the camera interface: read the camera and choose a view.
pub const IFACE_CAMERA: &str = "bb.camera";
/// The version the host implements for [`IFACE_CAMERA`], as `(major, minor)`.
pub const IFACE_CAMERA_VERSION: (u16, u16) = (1, 1);

/// Id of the audio interface: play one-shot sounds.
pub const IFACE_AUDIO: &str = "bb.audio";
/// The version the host implements for [`IFACE_AUDIO`], as `(major, minor)`.
pub const IFACE_AUDIO_VERSION: (u16, u16) = (1, 0);

/// Id of the UI interface: named themes a mod registers, and the draw specs of their slots.
pub const IFACE_UI: &str = "bb.ui";
/// The version the host implements for [`IFACE_UI`], as `(major, minor)`.
pub const IFACE_UI_VERSION: (u16, u16) = (1, 1);

/// Id of the translation interface: the mod's own messages in the player's language.
pub const IFACE_I18N: &str = "bb.i18n";
/// The version the host implements for [`IFACE_I18N`], as `(major, minor)`.
pub const IFACE_I18N_VERSION: (u16, u16) = (1, 0);

/// Id of the world interface: the track, the district, the rain and the traffic.
pub const IFACE_WORLD: &str = "bb.world";
/// The version the host implements for [`IFACE_WORLD`], as `(major, minor)`.
pub const IFACE_WORLD_VERSION: (u16, u16) = (1, 0);

/// Id of the configuration interface: the mod's own config file merged with its table in the game's main config.
pub const IFACE_CONFIG: &str = "bb.config";
/// The version the host implements for [`IFACE_CONFIG`], as `(major, minor)`.
pub const IFACE_CONFIG_VERSION: (u16, u16) = (1, 0);

/// Id of the engine interface: read the player's pedals and switch the player's engine off and on.
pub const IFACE_ENGINE: &str = "bb.engine";
/// The version the host implements for [`IFACE_ENGINE`], as `(major, minor)`.
pub const IFACE_ENGINE_VERSION: (u16, u16) = (1, 0);

/// Id of the traffic interface: the traffic cars that mods added, listed and placed on the road.
pub const IFACE_TRAFFIC: &str = "bb.traffic";
/// The version the host implements for [`IFACE_TRAFFIC`], as `(major, minor)`.
pub const IFACE_TRAFFIC_VERSION: (u16, u16) = (1, 0);
