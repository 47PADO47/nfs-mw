//! The interface registry: the generic interfaces the host offers, the version each one implements, and the builder that
//! makes one table of it for each mod (ADR 0005 section 1, "interfaces are the capability unit").
//!
//! A game builds a registry, adds the backends it implements, and passes it to `ModRuntime::start_with`. An interface the
//! game does not add is absent: a mod that asks for it gets null, and the SDK reports `None`.

use std::rc::Rc;

use super::manifest::InterfaceVersion;
use super::offer::InterfaceOffer;
use super::table::{
    Attached, CommandsBackend, InputBackend, MapBackend, MenusBackend, TimeBackend, attach_commands, attach_input,
    attach_map, attach_menus, attach_time,
};
use crate::abi::{
    IFACE_COMMANDS, IFACE_COMMANDS_VERSION, IFACE_INPUT, IFACE_INPUT_VERSION, IFACE_MAP, IFACE_MAP_VERSION,
    IFACE_MENUS, IFACE_MENUS_VERSION, IFACE_TIME, IFACE_TIME_VERSION,
};

/// Makes the table of one interface for the mod named by the argument.
type Build = Box<dyn Fn(&str) -> Attached>;

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
        let build: Build = Box::new(move |owner: &str| attach_commands(backend.clone(), owner));
        self.push(IFACE_COMMANDS, IFACE_COMMANDS_VERSION, build)
    }

    /// Offer `bb.input`, implemented by `backend`.
    pub fn add_input(&mut self, backend: Rc<dyn InputBackend>) -> &mut Self {
        let build: Build = Box::new(move |owner: &str| attach_input(backend.clone(), owner));
        self.push(IFACE_INPUT, IFACE_INPUT_VERSION, build)
    }

    /// Offer `bb.time`, implemented by `backend`.
    pub fn add_time(&mut self, backend: Rc<dyn TimeBackend>) -> &mut Self {
        let build: Build = Box::new(move |owner: &str| attach_time(backend.clone(), owner));
        self.push(IFACE_TIME, IFACE_TIME_VERSION, build)
    }

    /// Offer `bb.map`, implemented by `backend`.
    pub fn add_map(&mut self, backend: Rc<dyn MapBackend>) -> &mut Self {
        let build: Build = Box::new(move |owner: &str| attach_map(backend.clone(), owner));
        self.push(IFACE_MAP, IFACE_MAP_VERSION, build)
    }

    /// Offer `bb.menus`, implemented by `backend`.
    pub fn add_menus(&mut self, backend: Rc<dyn MenusBackend>) -> &mut Self {
        let build: Build = Box::new(move |owner: &str| attach_menus(backend.clone(), owner));
        self.push(IFACE_MENUS, IFACE_MENUS_VERSION, build)
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
    pub(crate) fn build(&self, owner: &str, id: &str, major: u16, min_minor: u16) -> Option<Attached> {
        let provider = self.providers.iter().find(|provider| provider.id == id)?;
        if provider.version.major != major || provider.version.minor < min_minor {
            return None;
        }
        Some((provider.build)(owner))
    }

    fn push(&mut self, id: &'static str, version: (u16, u16), build: Build) -> &mut Self {
        self.providers.push(Provider { id, version: InterfaceVersion { major: version.0, minor: version.1 }, build });
        self
    }
}
