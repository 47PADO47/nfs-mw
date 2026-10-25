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

mod audio;
mod camera;
mod commands;
mod content;
mod ctx;
mod error;
mod event;
#[doc(hidden)]
pub mod ffi;
mod guard;
mod handle;
mod hud;
mod i18n;
mod iface;
mod input;
mod macros;
mod map;
mod menus;
mod module;
mod time;
mod ui;
mod vehicles;
mod world;

pub use crate::declare_mod;
pub use audio::Audio;
pub use camera::{Camera, CameraKind, CameraState};
pub use commands::Commands;
pub use content::Content;
pub use ctx::{Ctx, Level, MAX_MESSAGE_BYTES};
pub use error::{Error, ModResult};
pub use event::{Collider, Event, Role};
pub use handle::{AbilityId, ActionId, BlipId, CommandId, MenuId, SoundId, UiThemeId, VehicleId, WorldId};
pub use hud::{Hud, MessageStyle};
pub use i18n::{Arg as I18nArg, I18n};
pub use iface::Interface;
pub use input::Input;
pub use map::Map;
pub use menus::{Menus, SliderRange};
pub use module::Mod;
pub use time::{Time, TimeOfDay};
pub use ui::{Prim, Rect, Tokens, Ui};
pub use vehicles::{VehicleState, Vehicles};
pub use world::World;

/// The SDK version, reported to the host by `bb_mod_info`.
pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");
