//! The SDK a mod links: the safe face of the ABI (ADR 0005 section 2).
//!
//! A mod implements [`Mod`] and calls `declare_mod!` once. The macro exports the two entry symbols. Every callback
//! runs inside a panic guard, so a panic becomes an error status and never unwinds into the host. The unsafe
//! pointer work lives in [`ffi`] and nowhere else in the SDK.
//!
//! ```ignore
//! use blackbox_modding::sdk::{declare_mod, Ctx, Event, Level, Mod, ModResult};
//!
//! struct Hello;
//!
//! impl Mod for Hello {
//!     fn load(ctx: &mut Ctx) -> ModResult<Self> {
//!         ctx.log(Level::Info, "hello loaded");
//!         Ok(Hello)
//!     }
//! }
//!
//! declare_mod!(Hello);
//! ```

#[cfg(panic = "abort")]
compile_error!("blackbox-modding mods must be built with panic = \"unwind\"; a panic has to reach the SDK guard");

mod ctx;
mod error;
mod event;
#[doc(hidden)]
pub mod ffi;
mod guard;
mod iface;
mod macros;
mod module;

pub use crate::declare_mod;
pub use ctx::{Ctx, Level, MAX_MESSAGE_BYTES};
pub use error::{Error, ModResult};
pub use event::Event;
pub use iface::Interface;
pub use module::Mod;

/// The SDK version, reported to the host by `bb_mod_info`.
pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");
