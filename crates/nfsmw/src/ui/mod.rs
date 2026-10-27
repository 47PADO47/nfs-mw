//! The shared FEng host layer: the packages of the install, the fonts and textures they draw with, and the one
//! presenter that turns a [`blackbox_feng::UiTree`] into UI meshes. The HUD and the menus both sit on it
//! (docs/decisions/0002-ui-presentation.md).

mod assets;
pub(crate) use assets::{decode_png_rgba, read_wanted};
mod catalog;
pub(crate) mod discord_icon;
pub mod fonts;
pub(crate) mod input_icons;
mod late_textures;
mod loading;
pub(crate) mod mod_icon;
pub(crate) mod name_picture;
pub mod present;
pub(crate) mod prompts;
mod raster_text;
mod shared;
mod strings;
mod system_fonts;
mod text;
pub mod theme;
pub(crate) mod tips;

pub use assets::UiAssets;
pub use catalog::{Catalog, SCREEN_FILES};
pub use loading::{AssetsLoad, Job, UiLoading};
pub use shared::{Presenter, SharedAssets, ensure};
pub use strings::Text;
pub use text::text_size;

/// The PC UI service uses 1200 authored ticks per real second.
/// Spec: `docs/specs/feng-runtime.md`. Every label reads from the current language (`crate::i18n`).
pub(crate) fn runtime() -> blackbox_feng::Runtime {
    let mut runtime = blackbox_feng::Runtime::with_tick_rate(1200.0);
    runtime.set_string_resolver(crate::i18n::game);
    runtime
}

/// Relabels a runtime's packages after the language changes. Each runtime owner keeps one and calls it every frame.
#[derive(Debug, Default)]
pub(crate) struct LanguageWatch {
    seen: u64,
}

impl LanguageWatch {
    pub(crate) fn apply(&mut self, runtime: &mut blackbox_feng::Runtime) {
        let now = crate::i18n::generation();
        if self.seen == now {
            return;
        }
        self.seen = now;
        // Setting the resolver again marks every package: each label is looked up in the new language.
        runtime.set_string_resolver(crate::i18n::game);
    }
}
