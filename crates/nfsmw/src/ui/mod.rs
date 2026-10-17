//! The shared FEng host layer: the packages of the install, the fonts and textures they draw with, and the one
//! presenter that turns a [`blackbox_feng::UiTree`] into UI meshes. The HUD and the menus both sit on it
//! (docs/decisions/0002-ui-presentation.md).

mod assets;
mod catalog;
pub(crate) mod discord_icon;
pub mod fonts;
pub(crate) mod input_icons;
pub(crate) mod mod_icon;
pub mod present;
pub(crate) mod prompts;
mod shared;
mod strings;
mod text;

pub use assets::UiAssets;
pub use catalog::{Catalog, SCREEN_FILES};
pub use shared::{Presenter, SharedAssets, ensure};
pub use strings::Text;
pub use text::text_size;

/// The PC UI service uses 1200 authored ticks per real second.
/// Spec: `docs/specs/feng-runtime.md`.
pub(crate) fn runtime() -> blackbox_feng::Runtime {
    blackbox_feng::Runtime::with_tick_rate(1200.0)
}
