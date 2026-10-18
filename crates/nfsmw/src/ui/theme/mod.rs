//! The MW side of the themeable UI: the element catalogue ([`ElementId`]), the two built-in themes, and the
//! registry that holds them as a Bevy resource.
//!
//! The engine-generic part (components, `Theme`, `Themes`, the resolver) is `libs/blackbox-ui`. This module
//! decides which game element is which slot and which theme fills it. Design: `docs/specs/ui-themes.md`.
//!
//! Status (T12): [`UiThemes`] is inserted at start-up, and the HUD and menu presenters resolve every slot through the
//! composer ([`compose`]). The Original theme fills every slot in the catalogue, and [`build_themes`] checks that. The
//! Custom theme fills the radio card (T9). The active theme follows `settings.ui_style` each frame
//! ([`sync_active_theme`]), and a mod theme follows the mods' registrations ([`sync_mod_themes`]).
//!
//! - [`element`]: the [`ElementId`] catalogue with its explicit numbers.
//! - [`compose`]: builds each presented layer from the slots its theme resolves (`docs/specs/ui-themes-compose.md`).
//! - [`original`]: the floor theme. It fills every slot: HUD adapters (T7) and menu adapters (T8).
//! - [`custom`]: the rewrite's own theme. It fills the radio card (T9) and returns `None` for the rest.
//! - [`modded`]: the themes the mods register through `bb.ui`. A mod theme is selected with [`UiThemes::select_mod`], by the
//!   Mods page's theme choice through [`UiThemes::set_mod_choice`]; the style setting does not choose it
//!   (`docs/specs/ui-themes.md` section 12).

mod compose;
#[cfg(test)]
mod compose_tests;
pub mod custom;
mod element;
pub mod modded;
pub mod original;
#[cfg(test)]
mod tests;

pub use compose::{hud_layer, slot_layer};

pub use element::ElementId;

use bevy_ecs::prelude::*;
use blackbox_ui::{BuildCx, Component, DrawList, LayoutCx, Rect, Skin, Theme, ThemeError, ThemeId, Themes};

use crate::settings::{Settings, UiStyle};
use custom::{CustomTheme, RadioFeedCell};
use modded::ModTheme;
use original::{FrameCell, OriginalTheme};

/// The theme a UI style selects. `Remastered` has no theme yet: [`UiThemes::select`] then keeps Original active.
pub fn theme_of(style: UiStyle) -> ThemeId {
    match style {
        UiStyle::Original => ThemeId::Original,
        UiStyle::Custom => ThemeId::Custom,
        UiStyle::Remastered => ThemeId::Remastered,
    }
}

/// Keeps the active theme in step with `settings.ui_style`, every frame, so an options-menu change takes effect on
/// the next frame (`docs/specs/ui-themes.md` section 14, question 5). A chosen mod theme is kept: [`UiThemes::select`]
/// leaves it active until the choice is cleared.
pub fn sync_active_theme(settings: Res<Settings>, mut themes: ResMut<UiThemes>) {
    // `bypass_change_detection`: the registry is selected every frame, so it should not look changed every frame.
    themes.bypass_change_detection().select(settings.ui_style);
}

/// Keeps the mod themes and the player's mod theme choice in step with the mods, every frame. The mods are a non-send
/// resource, so the system takes them as `NonSend`. A set that has not changed costs one comparison. The choice is made
/// on the Mods page (`frontend/mod_options.rs`) and kept by the bridge; [`sync_active_theme`] does not undo it.
pub fn sync_mod_themes(mods: Option<NonSend<crate::modding::Mods>>, mut themes: ResMut<UiThemes>) {
    let Some(mods) = mods else {
        return;
    };
    let themes = themes.bypass_change_detection();
    themes.set_mod_themes(mods.ui_themes());
    themes.set_mod_choice(mods.ui_theme_choice());
}

/// Builds the registry: Original as the floor, Custom registered beside it, Original active. The Original theme reads
/// its trees from `frame`, and the Custom theme its radio card from `radio`. The caller keeps a clone of each so it can
/// fill them each frame.
///
/// Panics when the Original theme leaves a catalogue slot unfilled. That is a programming error, found at start-up.
/// The generic `Themes::new` cannot make this check, because it does not know the MW catalogue
/// (`docs/specs/ui-themes.md` section 7).
pub fn build_themes(frame: FrameCell, radio: RadioFeedCell) -> Themes {
    check_floor_fills_every_slot(&frame);
    let mut themes = Themes::new(Box::new(OriginalTheme::new(frame))).expect("OriginalTheme reports ThemeId::Original");
    themes.register(Box::new(CustomTheme::new(radio))).expect("CustomTheme is registered once, and is not Original");
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

/// The skin the themes hand to their components: the Custom card's colours and font (T9). The Original adapters read no
/// skin key yet, so the card's keys sit alone in it.
pub fn default_skin() -> Skin {
    custom::card_skin()
}

/// The theme registry, the skin and the cells, as one Bevy resource. The HUD and the menus resolve their slots through it
/// each frame (the composer, `compose.rs`).
#[derive(Resource)]
pub struct UiThemes {
    themes: Themes,
    skin: Skin,
    frame: FrameCell,
    radio: RadioFeedCell,
    /// The style the settings chose. A chosen mod theme wins over it.
    style: UiStyle,
    /// The mod theme the player chose (see [`UiThemes::set_mod_choice`]), or `None` to follow `style`.
    mod_choice: Option<ThemeId>,
    /// The themes the mods registered, as the registry holds them (rebuilt into `themes` when they change).
    mods: Vec<ModTheme>,
}

impl UiThemes {
    /// The registry from [`build_themes`] with [`default_skin`], and empty cells.
    pub fn build() -> Self {
        let frame = FrameCell::new();
        let radio = RadioFeedCell::new();
        Self {
            themes: build_themes(frame.clone(), radio.clone()),
            skin: default_skin(),
            frame,
            radio,
            style: UiStyle::Original,
            mod_choice: None,
            mods: Vec::new(),
        }
    }

    /// Makes the theme for `style` active, unless a mod theme was chosen. A theme that is not registered (Remastered, for
    /// now) leaves Original active, so the registry never names a theme that nothing can fill.
    pub fn select(&mut self, style: UiStyle) {
        self.style = style;
        self.apply();
    }

    /// Makes the mod theme `id` active, until [`UiThemes::clear_mod`]. Refuses an id no mod registered.
    pub fn select_mod(&mut self, id: ThemeId) -> Result<(), ThemeError> {
        if !self.mods.iter().any(|theme| theme.id() == id) {
            return Err(ThemeError::Unknown(id));
        }
        self.mod_choice = Some(id);
        self.apply();
        Ok(())
    }

    /// Gives up the mod theme chosen with [`UiThemes::select_mod`]: the style decides again.
    pub fn clear_mod(&mut self) {
        self.mod_choice = None;
        self.apply();
    }

    /// Sets the mod theme choice from the bridge (`None` for the built-in style), once per frame. A choice whose theme is
    /// not registered is given up, so the style decides until the theme registers again; the saved choice then selects it.
    pub fn set_mod_choice(&mut self, choice: Option<ThemeId>) {
        let Some(id) = choice else {
            self.clear_mod();
            return;
        };
        if self.select_mod(id).is_err() {
            self.clear_mod();
        }
    }

    /// Replaces the mod themes with `mods`. An unchanged set costs one comparison. A changed set rebuilds the registry
    /// (the Original and Custom themes come back as they were), and a chosen mod theme that is gone is dropped.
    pub fn set_mod_themes(&mut self, mods: Vec<ModTheme>) {
        if self.mods == mods {
            return;
        }
        self.mods = mods;
        self.mod_choice = self.mod_choice.filter(|id| self.mods.iter().any(|theme| theme.id() == *id));
        self.themes = build_themes(self.frame.clone(), self.radio.clone());
        for theme in &self.mods {
            // The port gives each mod theme its own id, so a refusal here is a bug. It is logged and the theme skipped.
            if let Err(error) = self.themes.register(Box::new(theme.clone())) {
                log::warn!("the mod theme {:?} is not registered: {error}", theme.id());
            }
        }
        self.apply();
    }

    /// The active theme is the chosen mod theme, else the one the style names; a theme that is not registered leaves
    /// Original active.
    fn apply(&mut self) {
        let wanted = self.mod_choice.unwrap_or(theme_of(self.style));
        if self.themes.set_active(wanted).is_ok() {
            return;
        }
        self.themes.set_active(ThemeId::Original).expect("the Original theme is always registered");
    }

    /// The component for `slot` from the active theme, falling back to Original.
    pub fn resolve(&self, slot: ElementId) -> Box<dyn Component> {
        self.themes.resolve(slot.slot(), &BuildCx::new(&self.skin))
    }

    /// The theme whose component draws `slot` now: the active theme's when it provides the slot, else Original's.
    /// `None` when no theme provides it. The HUD presenter reads this for the RadioCard slot, to choose the chyron.
    pub fn provided_by(&self, slot: ElementId) -> Option<ThemeId> {
        self.themes.provider(slot.slot(), &BuildCx::new(&self.skin))
    }

    /// Resolves `slot` and draws it into `out`: the component is laid out at the screen's bounds, at `time_s` on the
    /// host clock, then built. A component that draws nothing adds no node.
    pub(crate) fn build_slot(&self, slot: ElementId, time_s: f32, out: &mut DrawList) {
        let mut component = self.resolve(slot);
        component.layout(&LayoutCx::new(&self.skin, time_s), Rect::SCREEN);
        component.build(out);
    }

    /// The frame cells the Original adapters read: the HUD trees and the menu trees. The HUD and frontend plugins keep
    /// this clone and call `set` / `set_menu` on it once per frame, after `Runtime::tree`.
    pub fn frame(&self) -> FrameCell {
        self.frame.clone()
    }

    /// The radio feed the Custom card reads. The HUD plugin keeps this clone and calls `set` on it once per frame, after
    /// `radio::update`.
    pub fn radio(&self) -> RadioFeedCell {
        self.radio.clone()
    }
}
