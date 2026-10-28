//! The interface registry: the generic interfaces the host offers, the version each one implements, and the builder that
//! makes one table of it for each mod (ADR 0005 section 1, "interfaces are the capability unit").
//!
//! A game builds a registry, adds the backends it implements, and passes it to `ModRuntime::start_with`. An interface the
//! game does not add is absent: a mod that asks for it gets null, and the SDK reports `None`. A game that offers an
//! interface of its own (the `nfsmw.*` ones, for example) adds a provider with [`InterfaceRegistry::add_provider`].

use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::manifest::InterfaceVersion;
use super::offer::InterfaceOffer;
use super::table::{
    Attached, AudioBackend, CameraBackend, CommandsBackend, ConfigBackend, ContentBackend, HudBackend, I18nBackend,
    InputBackend, MapBackend, MenusBackend, SettingsBackend, TimeBackend, UiBackend, VehiclesBackend, WorldBackend,
    attach_audio, attach_camera, attach_commands, attach_config, attach_content, attach_hud, attach_i18n, attach_input,
    attach_map, attach_menus, attach_settings, attach_time, attach_ui, attach_vehicles, attach_world,
};
use super::table::{EngineBackend, attach_engine};
use super::table::{TrafficBackend, attach_traffic};
use crate::abi::{
    IFACE_AUDIO, IFACE_AUDIO_VERSION, IFACE_CAMERA, IFACE_CAMERA_VERSION, IFACE_COMMANDS, IFACE_COMMANDS_VERSION,
    IFACE_CONFIG, IFACE_CONFIG_VERSION, IFACE_CONTENT, IFACE_CONTENT_VERSION, IFACE_HUD, IFACE_HUD_VERSION, IFACE_I18N,
    IFACE_I18N_VERSION, IFACE_INPUT, IFACE_INPUT_VERSION, IFACE_MAP, IFACE_MAP_VERSION, IFACE_MENUS,
    IFACE_MENUS_VERSION, IFACE_SETTINGS, IFACE_SETTINGS_VERSION, IFACE_TIME, IFACE_TIME_VERSION, IFACE_UI,
    IFACE_UI_VERSION, IFACE_VEHICLES, IFACE_VEHICLES_VERSION, IFACE_WORLD, IFACE_WORLD_VERSION,
};
use crate::abi::{IFACE_ENGINE, IFACE_ENGINE_VERSION};
use crate::abi::{IFACE_TRAFFIC, IFACE_TRAFFIC_VERSION};

/// Where a mod lives: its folder, and the manifest's `config` value (its own config file, relative to the folder).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModPlace {
    pub(crate) dir: PathBuf,
    pub(crate) config_file: Option<String>,
}

/// What a provider knows when it builds the table of one mod: the mod's id, and where it lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostCtx<'a> {
    owner: &'a str,
    place: &'a ModPlace,
}

impl<'a> HostCtx<'a> {
    /// A context for the mod named `owner` living at `place`.
    pub(crate) fn new(owner: &'a str, place: &'a ModPlace) -> HostCtx<'a> {
        HostCtx { owner, place }
    }

    /// The folder of the mod the table is built for.
    pub fn mod_dir(&self) -> &Path {
        &self.place.dir
    }

    /// The mod's own config file as its manifest names it (`[mod] config`), relative to [`HostCtx::mod_dir`]; `None` for the
    /// default, `config.toml`.
    pub fn config_file(&self) -> Option<&str> {
        self.place.config_file.as_deref()
    }

    /// The id of the mod the table is built for. The provider passes it to its backend, as the generic ones do.
    pub fn owner(&self) -> &str {
        self.owner
    }
}

/// Makes the table of one interface for the mod the context names.
type Build = Box<dyn Fn(&HostCtx<'_>) -> Attached>;

/// One offered interface: its id, its version, and how to build its table.
struct Provider {
    id: &'static str,
    version: InterfaceVersion,
    build: Build,
}

/// The interfaces a host offers. Ids are unique: when two providers share an id, the first one added wins.
#[derive(Default)]
pub struct InterfaceRegistry {
    providers: Vec<Provider>,
}

impl InterfaceRegistry {
    /// An empty registry: the host offers no interface, so every mod runs with its own logging only.
    pub fn new() -> Self {
        Self::default()
    }

    /// Offer `bb.commands`, implemented by `backend`.
    pub fn add_commands(&mut self, backend: Rc<dyn CommandsBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_commands(backend.clone(), ctx.owner()));
        self.push(IFACE_COMMANDS, IFACE_COMMANDS_VERSION, build)
    }

    /// Offer `bb.input`, implemented by `backend`.
    pub fn add_input(&mut self, backend: Rc<dyn InputBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_input(backend.clone(), ctx.owner()));
        self.push(IFACE_INPUT, IFACE_INPUT_VERSION, build)
    }

    /// Offer `bb.config`, implemented by `backend`: the game's half of a mod's configuration (the mod's own file is read by the
    /// host from the mod's folder).
    pub fn add_config(&mut self, backend: Rc<dyn ConfigBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| {
            attach_config(backend.clone(), ctx.owner(), ctx.mod_dir().to_owned(), ctx.config_file().map(str::to_owned))
        });
        self.push(IFACE_CONFIG, IFACE_CONFIG_VERSION, build)
    }

    /// Offer `bb.content`, implemented by `backend`.
    pub fn add_content(&mut self, backend: Rc<dyn ContentBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_content(backend.clone(), ctx.owner()));
        self.push(IFACE_CONTENT, IFACE_CONTENT_VERSION, build)
    }

    /// Offer `bb.time`, implemented by `backend`.
    pub fn add_time(&mut self, backend: Rc<dyn TimeBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_time(backend.clone(), ctx.owner()));
        self.push(IFACE_TIME, IFACE_TIME_VERSION, build)
    }

    /// Offer `bb.engine`, implemented by `backend`.
    pub fn add_engine(&mut self, backend: Rc<dyn EngineBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_engine(backend.clone(), ctx.owner()));
        self.push(IFACE_ENGINE, IFACE_ENGINE_VERSION, build)
    }

    /// Offer `bb.traffic`, implemented by `backend`.
    pub fn add_traffic(&mut self, backend: Rc<dyn TrafficBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_traffic(backend.clone(), ctx.owner()));
        self.push(IFACE_TRAFFIC, IFACE_TRAFFIC_VERSION, build)
    }

    /// Offer `bb.map`, implemented by `backend`.
    pub fn add_map(&mut self, backend: Rc<dyn MapBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_map(backend.clone(), ctx.owner()));
        self.push(IFACE_MAP, IFACE_MAP_VERSION, build)
    }

    /// Offer `bb.i18n`, implemented by `backend`.
    pub fn add_i18n(&mut self, backend: Rc<dyn I18nBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_i18n(backend.clone(), ctx.owner()));
        self.push(IFACE_I18N, IFACE_I18N_VERSION, build)
    }

    /// Offer `bb.menus`, implemented by `backend`.
    pub fn add_menus(&mut self, backend: Rc<dyn MenusBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_menus(backend.clone(), ctx.owner()));
        self.push(IFACE_MENUS, IFACE_MENUS_VERSION, build)
    }

    /// Offer `bb.settings`, implemented by `backend`.
    pub fn add_settings(&mut self, backend: Rc<dyn SettingsBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_settings(backend.clone(), ctx.owner()));
        self.push(IFACE_SETTINGS, IFACE_SETTINGS_VERSION, build)
    }

    /// Offer `bb.ui`, implemented by `backend`.
    pub fn add_ui(&mut self, backend: Rc<dyn UiBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_ui(backend.clone(), ctx.owner()));
        self.push(IFACE_UI, IFACE_UI_VERSION, build)
    }

    /// Offer `bb.vehicles`, implemented by `backend`.
    pub fn add_vehicles(&mut self, backend: Rc<dyn VehiclesBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_vehicles(backend.clone(), ctx.owner()));
        self.push(IFACE_VEHICLES, IFACE_VEHICLES_VERSION, build)
    }

    /// Offer `bb.hud`, implemented by `backend`.
    pub fn add_hud(&mut self, backend: Rc<dyn HudBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_hud(backend.clone(), ctx.owner()));
        self.push(IFACE_HUD, IFACE_HUD_VERSION, build)
    }

    /// Offer `bb.camera`, implemented by `backend`.
    pub fn add_camera(&mut self, backend: Rc<dyn CameraBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_camera(backend.clone(), ctx.owner()));
        self.push(IFACE_CAMERA, IFACE_CAMERA_VERSION, build)
    }

    /// Offer `bb.audio`, implemented by `backend`.
    pub fn add_audio(&mut self, backend: Rc<dyn AudioBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_audio(backend.clone(), ctx.owner()));
        self.push(IFACE_AUDIO, IFACE_AUDIO_VERSION, build)
    }

    /// Offer `bb.world`, implemented by `backend`.
    pub fn add_world(&mut self, backend: Rc<dyn WorldBackend>) -> &mut Self {
        let build: Build = Box::new(move |ctx: &HostCtx<'_>| attach_world(backend.clone(), ctx.owner()));
        self.push(IFACE_WORLD, IFACE_WORLD_VERSION, build)
    }

    /// Offer interface `id` at `version`, whose table `provider` builds for each mod that asks for it. The provider gets
    /// the mod's id in its [`HostCtx`] and answers with the table it made, wrapped by [`Attached::from_owned`]. An id
    /// already offered keeps its first provider.
    pub fn add_provider(
        &mut self,
        id: &'static str,
        version: (u16, u16),
        provider: impl Fn(&HostCtx<'_>) -> Attached + 'static,
    ) -> &mut Self {
        self.push(id, version, Box::new(provider))
    }

    /// The interfaces offered, for the host's offer (the manifest checks read it).
    pub fn offers(&self) -> Vec<InterfaceOffer> {
        self.providers
            .iter()
            .map(|provider| InterfaceOffer { id: provider.id.to_owned(), version: provider.version })
            .collect()
    }

    /// The table of interface `id` for mod `owner`, when the registry offers it at `major` with a minor of at least
    /// `min_minor`.
    #[cfg(test)]
    pub(crate) fn build(&self, owner: &str, id: &str, major: u16, min_minor: u16) -> Option<Attached> {
        self.build_in(&HostCtx::new(owner, &ModPlace::default()), id, major, min_minor)
    }

    /// The table of interface `id` for the mod `ctx` describes, when the registry offers it at `major` with a minor of at
    /// least `min_minor`.
    pub(crate) fn build_in(&self, ctx: &HostCtx<'_>, id: &str, major: u16, min_minor: u16) -> Option<Attached> {
        let provider = self.providers.iter().find(|provider| provider.id == id)?;
        if provider.version.major != major || provider.version.minor < min_minor {
            return None;
        }
        Some((provider.build)(ctx))
    }

    fn push(&mut self, id: &'static str, version: (u16, u16), build: Build) -> &mut Self {
        self.providers.push(Provider { id, version: InterfaceVersion { major: version.0, minor: version.1 }, build });
        self
    }
}
