//! The host: the loader and dispatcher (ADR 0005 section 3). Feature `host`.
//!
//! The steps, in order, each a separate module:
//!
//! - [`manifest`]: parse and check `mod.toml`.
//! - [`discovery`]: scan the mod roots, apply the [`policy`] and the host's [`offer`].
//! - [`order`]: resolve dependencies and load-order hints; reject cycles and missing dependencies.
//! - [`loader`]: open one library, read `bb_mod_info`, handshake, `bb_mod_create`, `destroy`. The only module that calls
//!   into a mod.
//! - [`runtime`]: the start-up sequence, event dispatch with the fault counter ([`fault`]), dev reload ([`reload`]) and
//!   unload.
//!
//! The host names no game. The game supplies its identity and offer, and the backends of the generic interfaces it
//! implements, through an [`InterfaceRegistry`].
//!
//! - [`registry`]: the interfaces the host offers, and the per-mod tables it builds for them.
//! - `table`: the host table of each mod, `query_interface`, and the trampolines behind the interface tables.

pub mod discovery;
pub mod fault;
pub mod gate;
pub mod loader;
pub mod manifest;
pub mod offer;
pub mod order;
pub mod policy;
pub mod registry;
pub mod reject;
pub mod reload;
pub mod runtime;
mod table;

#[cfg(test)]
pub(crate) mod test_support;

pub use discovery::{Candidate, Discovery, check_offer, discover};
pub use fault::{FaultCounter, QUARANTINE_AFTER};
pub use loader::{LoadError, Module};
pub use manifest::{MANIFEST_FILE, Manifest, ManifestError, parse};
pub use offer::{HostOffer, InterfaceOffer};
pub use order::{Resolution, resolve};
pub use policy::Policy;
pub use registry::{HostCtx, InterfaceRegistry};
pub use reject::{RejectReason, Rejected};
pub use runtime::{DispatchReport, ModRuntime, ModSummary, ReloadError, RunMode};
pub use table::{
    Attached, AudioBackend, BackendResult, BlipSpec, CameraBackend, CameraKind, CameraState, CommandHandler,
    CommandsBackend, HotkeyHandler, HudBackend, InputBackend, MAX_CHOICES, MAX_COP_SHARE, MAX_MESSAGE_SECS, MapBackend,
    MenuHandler, MenuKind, MenuRowSpec, MenusBackend, MessageStyle, TimeBackend, VehicleState, VehiclesBackend,
    WorldBackend,
};
