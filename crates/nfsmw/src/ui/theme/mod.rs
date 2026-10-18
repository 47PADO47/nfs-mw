//! The MW side of the themeable UI: the element catalogue ([`ElementId`]), the two built-in themes, and the
//! registry that holds them as a Bevy resource.
//!
//! The engine-generic part (components, `Theme`, `Themes`, the resolver) is `libs/blackbox-ui`. This module
//! decides which game element is which slot and which theme fills it. Design: `docs/specs/ui-themes.md`.
//!
//! Status (T8): [`UiThemes`] is inserted at start-up. The Original theme fills every slot in the catalogue (HUD in T7,
//! menus and career screens in T8), and [`build_themes`] checks that. Its adapters read the frame cells
//! ([`UiThemes::frame`]), but nothing resolves a slot in the live path yet. The presenter wiring comes in T12, so the
//! game draws exactly as it did before this module existed.
//!
//! - [`element`]: the [`ElementId`] catalogue with its explicit numbers.
//! - [`original`]: the floor theme. It fills every slot: HUD adapters (T7) and menu adapters (T8).
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
/// its trees from `frame`, which the caller keeps a clone of so it can fill the cells each frame.
///
/// Panics when the Original theme leaves a catalogue slot unfilled. That is a programming error, found at start-up.
/// The generic `Themes::new` cannot make this check, because it does not know the MW catalogue
/// (`docs/specs/ui-themes.md` section 7).
pub fn build_themes(frame: FrameCell) -> Themes {
    check_floor_fills_every_slot(&frame);
    let mut themes = Themes::new(Box::new(OriginalTheme::new(frame))).expect("OriginalTheme reports ThemeId::Original");
    themes.register(Box::new(CustomTheme)).expect("CustomTheme is registered once, and is not Original");
    themes
}

/// Asserts that the Original theme returns a component for every slot in [`ElementId::ALL`].
fn check_floor_fills_every_slot(frame: &FrameCell) {
    let floor = OriginalTheme::new(frame.clone());
    let skin = default_skin();
    let cx = BuildCx::new(&skin);
    for element in ElementId::ALL {
        let filled = blackbox_ui::Theme::element(&floor, element.slot(), &cx).is_some();
        assert!(filled, "the Original theme fills no component for the {} slot", element.name());
    }
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

    /// The frame cells the Original adapters read: the HUD trees and the menu trees. The HUD and frontend plugins keep
    /// this clone and call `set` / `set_menu` on it once per frame, after `Runtime::tree` (T12). Until then the cells are
    /// empty and the adapters draw nothing.
    // wired into the HUD and frontend plugins in T12.
    #[allow(dead_code)]
    pub fn frame(&self) -> FrameCell {
        self.frame.clone()
    }
}
