//! `Ctx`: the mod's view of the host, handed to every callback.

use core::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use super::ffi::{self, HostTable};
use super::{
    Audio, Camera, Commands, Content, Engine, Hud, I18n, Input, Interface, Map, Menus, ModConfig, ModResult, Settings,
    Time, Traffic, Ui, Vehicles, World,
};
use crate::abi::{
    AbiVersion, BB_LOG_DEBUG, BB_LOG_ERROR, BB_LOG_INFO, BB_LOG_TRACE, BB_LOG_WARN, IFACE_AUDIO, IFACE_AUDIO_VERSION,
    IFACE_CAMERA, IFACE_CAMERA_VERSION, IFACE_COMMANDS, IFACE_COMMANDS_VERSION, IFACE_CONFIG, IFACE_CONFIG_VERSION,
    IFACE_CONTENT, IFACE_CONTENT_VERSION, IFACE_HUD, IFACE_HUD_VERSION, IFACE_I18N, IFACE_I18N_VERSION, IFACE_INPUT,
    IFACE_INPUT_VERSION, IFACE_MAP, IFACE_MAP_VERSION, IFACE_MENUS, IFACE_MENUS_VERSION, IFACE_SETTINGS,
    IFACE_SETTINGS_VERSION, IFACE_TIME, IFACE_TIME_VERSION, IFACE_UI, IFACE_UI_VERSION, IFACE_VEHICLES,
    IFACE_VEHICLES_VERSION, IFACE_WORLD, IFACE_WORLD_VERSION,
};
use crate::abi::{IFACE_ENGINE, IFACE_ENGINE_VERSION};
use crate::abi::{IFACE_TRAFFIC, IFACE_TRAFFIC_VERSION};

/// The longest message the host takes in one line (ADR 0005 section 4). Longer text is cut at a character boundary.
pub const MAX_MESSAGE_BYTES: usize = 1024;

/// Everything a mod keeps alive for the host: the Rust closures behind its registered commands and hotkeys. The host
/// calls them through C trampolines, so they must live until the mod unloads. The mod's slot clears them at `destroy`.
pub(crate) type Keep = Rc<RefCell<Vec<Box<dyn Any>>>>;

/// Log severity. Maps to the host's `BB_LOG_*` levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Level {
    /// Something failed.
    Error = BB_LOG_ERROR,
    /// Something is wrong but the game goes on.
    Warn = BB_LOG_WARN,
    /// Normal progress.
    Info = BB_LOG_INFO,
    /// Details for a developer.
    Debug = BB_LOG_DEBUG,
    /// Very detailed output.
    Trace = BB_LOG_TRACE,
}

/// The host as a mod sees it inside a callback. A `Ctx` cannot be stored or built outside the SDK, so it is
/// valid only for the callback that received it.
pub struct Ctx {
    host: HostTable,
    keep: Keep,
}

impl Ctx {
    /// Wrap the host table of the current mod.
    pub(crate) fn new(host: HostTable, keep: Keep) -> Self {
        Ctx { host, keep }
    }

    /// The ABI version the host reports (the one the handshake accepted).
    pub fn abi_version(&self) -> AbiVersion {
        self.host.version()
    }

    /// Write one line to the game log, cut to [`MAX_MESSAGE_BYTES`].
    pub fn log(&self, level: Level, msg: &str) {
        ffi::log(self.host, level as u32, clip(msg, MAX_MESSAGE_BYTES));
    }

    /// The game id the host reports, for example a short name of the game.
    pub fn game_id(&self) -> ModResult<String> {
        ffi::game_id(self.host)
    }

    /// The folder of this mod: where its `mod.toml` is. The mod's settings are read through [`Ctx::config`]. The content it
    /// generates (`files/`, `cars/`) is written here: the host reads those after the mod has loaded, so what a mod
    /// writes in `Mod::load` is part of the game's content for this run.
    pub fn mod_dir(&self) -> ModResult<std::path::PathBuf> {
        ffi::mod_dir(self.host).map(std::path::PathBuf::from)
    }

    /// The folder of the game's own install, or `Unsupported` when the host has none.
    pub fn game_dir(&self) -> ModResult<std::path::PathBuf> {
        let dir = ffi::game_dir(self.host)?;
        if dir.is_empty() {
            return Err(crate::sdk::Error::Unsupported);
        }
        Ok(std::path::PathBuf::from(dir))
    }

    /// The game version string the host reports.
    pub fn game_version(&self) -> ModResult<String> {
        ffi::game_version(self.host)
    }

    /// Ask the host for an interface by id. `None` when the host does not offer it, offers another major, or offers
    /// a minor below `min_minor`. A mod must cope with `None` for an optional interface.
    pub fn query_interface(&self, id: &str, major: u16, min_minor: u16) -> Option<Interface> {
        ffi::query_interface(self.host, id, major, min_minor)
    }

    /// The console commands interface (`bb.commands`), when the host offers it.
    pub fn commands(&self) -> Option<Commands> {
        let iface = self.query_interface(IFACE_COMMANDS, IFACE_COMMANDS_VERSION.0, IFACE_COMMANDS_VERSION.1)?;
        Some(Commands::new(ffi::commands::read(iface), self.host, self.keep.clone()))
    }

    /// The hotkey interface (`bb.input`), when the host offers it.
    pub fn input(&self) -> Option<Input> {
        let iface = self.query_interface(IFACE_INPUT, IFACE_INPUT_VERSION.0, IFACE_INPUT_VERSION.1)?;
        Some(Input::new(ffi::input::read(iface), self.host, self.keep.clone()))
    }

    /// The menus interface (`bb.menus`), when the host offers it: rows on the Mods page of the options.
    pub fn menus(&self) -> Option<Menus> {
        let iface = self.query_interface(IFACE_MENUS, IFACE_MENUS_VERSION.0, IFACE_MENUS_VERSION.1)?;
        Some(Menus::new(ffi::menus::read(iface), self.host, self.keep.clone()))
    }

    /// The settings interface (`bb.settings`), when the host offers it: options (and values of the game's options) in the
    /// gameplay settings.
    pub fn settings(&self) -> Option<Settings> {
        let iface = self.query_interface(IFACE_SETTINGS, IFACE_SETTINGS_VERSION.0, IFACE_SETTINGS_VERSION.1)?;
        Some(Settings::new(ffi::settings::read(iface), self.host, self.keep.clone()))
    }

    /// The UI interface (`bb.ui`), when the host offers it: named themes with draw specs for the slots they override.
    pub fn ui(&self) -> Option<Ui> {
        let iface = self.query_interface(IFACE_UI, IFACE_UI_VERSION.0, IFACE_UI_VERSION.1)?;
        Some(Ui::new(ffi::ui::read(iface)))
    }

    /// The content interface (`bb.content`), when the host offers it: how a mod that generates its content on a worker
    /// thread tells the host that it is ready.
    pub fn content(&self) -> Option<Content> {
        let iface = self.query_interface(IFACE_CONTENT, IFACE_CONTENT_VERSION.0, IFACE_CONTENT_VERSION.1)?;
        Some(Content::new(ffi::content::read(iface)))
    }

    /// The engine interface (`bb.engine`), when the host offers it: the player's pedals, and the engine's switch.
    pub fn engine(&self) -> Option<Engine> {
        let iface = self.query_interface(IFACE_ENGINE, IFACE_ENGINE_VERSION.0, IFACE_ENGINE_VERSION.1)?;
        Some(Engine::new(ffi::engine::read(iface)))
    }

    /// The traffic interface (`bb.traffic`), when the host offers it: the traffic cars that mods added, and a way to place one.
    pub fn traffic(&self) -> Option<Traffic> {
        let iface = self.query_interface(IFACE_TRAFFIC, IFACE_TRAFFIC_VERSION.0, IFACE_TRAFFIC_VERSION.1)?;
        Some(Traffic::new(ffi::traffic::read(iface)))
    }

    /// The time-of-day interface (`bb.time`), when the host offers it.
    pub fn time(&self) -> Option<Time> {
        let iface = self.query_interface(IFACE_TIME, IFACE_TIME_VERSION.0, IFACE_TIME_VERSION.1)?;
        Some(Time::new(ffi::time::read(iface)))
    }

    /// The map interface (`bb.map`), when the host offers it.
    pub fn map(&self) -> Option<Map> {
        let iface = self.query_interface(IFACE_MAP, IFACE_MAP_VERSION.0, IFACE_MAP_VERSION.1)?;
        Some(Map::new(ffi::map::read(iface)))
    }

    /// The vehicles interface (`bb.vehicles`), when the host offers it.
    pub fn vehicles(&self) -> Option<Vehicles> {
        let iface = self.query_interface(IFACE_VEHICLES, IFACE_VEHICLES_VERSION.0, IFACE_VEHICLES_VERSION.1)?;
        Some(Vehicles::new(ffi::vehicles::read(iface)))
    }

    /// The HUD interface (`bb.hud`), when the host offers it.
    pub fn hud(&self) -> Option<Hud> {
        let iface = self.query_interface(IFACE_HUD, IFACE_HUD_VERSION.0, IFACE_HUD_VERSION.1)?;
        Some(Hud::new(ffi::hud::read(iface)))
    }

    /// The camera interface (`bb.camera`), when the host offers it.
    pub fn camera(&self) -> Option<Camera> {
        let iface = self.query_interface(IFACE_CAMERA, IFACE_CAMERA_VERSION.0, IFACE_CAMERA_VERSION.1)?;
        Some(Camera::new(ffi::camera::read(iface), self.host, self.keep.clone()))
    }

    /// The audio interface (`bb.audio`), when the host offers it.
    pub fn audio(&self) -> Option<Audio> {
        let iface = self.query_interface(IFACE_AUDIO, IFACE_AUDIO_VERSION.0, IFACE_AUDIO_VERSION.1)?;
        Some(Audio::new(ffi::audio::read(iface)))
    }

    /// The world interface (`bb.world`), when the host offers it.
    pub fn world(&self) -> Option<World> {
        let iface = self.query_interface(IFACE_WORLD, IFACE_WORLD_VERSION.0, IFACE_WORLD_VERSION.1)?;
        Some(World::new(ffi::world::read(iface)))
    }

    /// The configuration interface (`bb.config`), when the host offers it: the mod's own config file merged with its table in
    /// the game's main config. Read it in `Mod::load`; the text does not change while the mod runs.
    pub fn config(&self) -> Option<ModConfig> {
        let iface = self.query_interface(IFACE_CONFIG, IFACE_CONFIG_VERSION.0, IFACE_CONFIG_VERSION.1)?;
        Some(ModConfig::new(ffi::config::read(iface)))
    }

    /// The translation interface (`bb.i18n`), when the host offers it.
    pub fn i18n(&self) -> Option<I18n> {
        let iface = self.query_interface(IFACE_I18N, IFACE_I18N_VERSION.0, IFACE_I18N_VERSION.1)?;
        Some(I18n::new(ffi::i18n::read(iface)))
    }
}

/// Cut `text` to at most `max` bytes, at a character boundary.
fn clip(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_unchanged() {
        assert_eq!(clip("abc", 8), "abc");
    }

    #[test]
    fn long_text_is_cut_at_a_character_boundary() {
        // Each "é" is two bytes, so a cut at byte 3 must back off to byte 2.
        assert_eq!(clip("ééé", 3), "é");
        assert_eq!(clip("ééé", 4), "éé");
    }

    #[test]
    fn levels_match_the_host_constants() {
        assert_eq!(Level::Error as u32, BB_LOG_ERROR);
        assert_eq!(Level::Trace as u32, BB_LOG_TRACE);
    }
}
