//! The MW side of the themeable UI: the element catalogue ([`ElementId`]), the two built-in themes, and the
//! registry that holds them as a Bevy resource.
//!
//! The engine-generic part (components, `Theme`, `Themes`, the resolver) is `libs/blackbox-ui`. This module
//! decides which game element is which slot and which theme fills it. Design: `docs/specs/ui-themes.md`.
//!
//! Status (T6, scaffolding): [`UiThemes`] is inserted at start-up, but nothing resolves a slot yet. The
//! presenter wiring comes in T12, so the game draws exactly as it did before this module existed.
//!
//! - [`element`]: the [`ElementId`] catalogue with its explicit numbers.
//! - [`original`]: the floor theme, which will wrap the existing HUD and menu code (T7, T8).
//! - [`custom`]: the rewrite's own theme, which will hold the radio card (T9).

pub mod custom;
mod element;
pub mod original;
#[cfg(test)]
mod tests;

pub use element::ElementId;

use bevy_ecs::prelude::*;
use blackbox_ui::{BuildCx, Component, Skin, Themes};

use custom::CustomTheme;
use original::OriginalTheme;

/// Builds the registry: Original as the floor, Custom registered beside it, Original active.
pub fn build_themes() -> Themes {
    let mut themes = Themes::new(Box::new(OriginalTheme)).expect("OriginalTheme reports ThemeId::Original");
    themes.register(Box::new(CustomTheme)).expect("CustomTheme is registered once, and is not Original");
    themes
}

/// The skin the themes hand to their components. Empty for now: T7 adds the original textures and fonts,
/// and T9 adds the custom card's colours and metrics.
pub fn default_skin() -> Skin {
    Skin::new()
}

/// The theme registry and the skin, as one Bevy resource. The HUD and the menus will resolve their slots
/// through it in T12.
#[derive(Resource)]
// wired into present in T12: nothing resolves a slot before then, so `resolve` and the fields look unused.
#[allow(dead_code)]
pub struct UiThemes {
    themes: Themes,
    skin: Skin,
}

impl UiThemes {
    /// The registry from [`build_themes`] with [`default_skin`].
    pub fn build() -> Self {
        Self { themes: build_themes(), skin: default_skin() }
    }

    /// The component for `slot` from the active theme, falling back to Original.
    // wired into present in T12: the presenter calls this for every HUD and menu slot.
    #[allow(dead_code)]
    pub fn resolve(&self, slot: ElementId) -> Box<dyn Component> {
        self.themes.resolve(slot.slot(), &BuildCx::new(&self.skin))
    }
}
