//! The MW side of the themeable UI: the element catalogue ([`ElementId`]), the two built-in themes, and the
//! registry that holds them as a Bevy resource.
//!
//! The engine-generic part (components, `Theme`, `Themes`, the resolver) is `libs/blackbox-ui`. This module
//! decides which game element is which slot and which theme fills it. Design: `docs/specs/ui-themes.md`.
//!
//! Status (T7): [`UiThemes`] is inserted at start-up. The Original HUD slots resolve to adapters that read the
//! frame cell ([`UiThemes::frame`]), but nothing resolves a slot in the live path yet. The presenter wiring comes in
//! T12, so the game draws exactly as it did before this module existed.
//!
//! - [`element`]: the [`ElementId`] catalogue with its explicit numbers.
//! - [`original`]: the floor theme. Its HUD adapters are filled (T7); its menu adapters follow (T8).
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
use original::{FrameCell, OriginalTheme};

/// Builds the registry: Original as the floor, Custom registered beside it, Original active. The Original theme reads
/// its trees from `frame`, which the caller keeps a clone of so it can fill the cell each frame.
pub fn build_themes(frame: FrameCell) -> Themes {
    let mut themes = Themes::new(Box::new(OriginalTheme::new(frame))).expect("OriginalTheme reports ThemeId::Original");
    themes.register(Box::new(CustomTheme)).expect("CustomTheme is registered once, and is not Original");
    themes
}

/// The skin the themes hand to their components. Empty for now: T7 adds the original textures and fonts,
/// and T9 adds the custom card's colours and metrics.
pub fn default_skin() -> Skin {
    Skin::new()
}

/// The theme registry, the skin and the frame cell, as one Bevy resource. The HUD and the menus will resolve their
/// slots through it in T12.
#[derive(Resource)]
// wired into present in T12: nothing resolves a slot before then, so `resolve`, `frame` and the fields look unused.
#[allow(dead_code)]
pub struct UiThemes {
    themes: Themes,
    skin: Skin,
    frame: FrameCell,
}

impl UiThemes {
    /// The registry from [`build_themes`] with [`default_skin`], and an empty frame cell.
    pub fn build() -> Self {
        let frame = FrameCell::new();
        Self { themes: build_themes(frame.clone()), skin: default_skin(), frame }
    }

    /// The component for `slot` from the active theme, falling back to Original.
    // wired into present in T12: the presenter calls this for every HUD and menu slot.
    #[allow(dead_code)]
    pub fn resolve(&self, slot: ElementId) -> Box<dyn Component> {
        self.themes.resolve(slot.slot(), &BuildCx::new(&self.skin))
    }

    /// The frame cell the Original HUD adapters read. The HUD plugin keeps this clone and calls `set` on it once per
    /// frame, after `Runtime::tree` (T12). Until then the cell is empty and the HUD adapters draw nothing.
    // wired into the HUD plugin in T12.
    #[allow(dead_code)]
    pub fn frame(&self) -> FrameCell {
        self.frame.clone()
    }
}
