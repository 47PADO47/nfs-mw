//! The shared FEng host layer: the packages of the install, the fonts and textures they draw with, and the one
//! presenter that turns a [`blackbox_feng::UiTree`] into UI meshes. The HUD and the menus both sit on it
//! (docs/decisions/0002-ui-presentation.md).

mod assets;
mod catalog;
pub mod present;
mod shared;
mod text;

pub use assets::UiAssets;
pub use catalog::{Catalog, SCREEN_FILES};
pub use shared::{Presenter, SharedAssets, ensure};
pub use text::text_size;
