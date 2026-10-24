//! The MW side of the themeable UI: the element catalogue ([`ElementId`]), the two built-in themes, and the
//! registry that holds them as a Bevy resource.
//!
//! The engine-generic part (components, `Theme`, `Themes`, the resolver) is `libs/blackbox-ui`. This module
//! decides which game element is which slot and which theme fills it. Design: `docs/specs/ui-themes.md`.
//!
//! Status (T12): [`UiThemes`] is inserted at start-up, and the HUD and menu presenters resolve every slot through the
//! composer ([`compose`]). The Original theme fills every slot in the catalogue, and [`build_themes`] checks that. The
//! Custom look (the radio card today, the Custom HUD 2026 of ADR 0013 as its components land) is drawn by the Custom
//! theme, and by a mod theme that declares tokens. The active theme follows `settings.ui_style` each frame
//! ([`sync_active_theme`]), and a mod theme follows the mods' registrations ([`sync_mod_themes`]). The active theme's
//! tokens, and the Custom HUD's `custom.*` tokens for the player's `[custom_hud]` look, are merged into the skin the
//! components read ([`UiThemes::refresh_skin`]).
//!
//! - [`element`]: the [`ElementId`] catalogue with its explicit numbers.
//! - [`compose`]: builds each presented layer from the slots its theme resolves (`docs/specs/ui-themes-compose.md`).
//! - [`original`]: the floor theme. It fills every slot: HUD adapters (T7) and menu adapters (T8).
//! - [`custom`]: the Custom look, its tokens, layout and feed, and its two palettes (Bevy, Remastered gold). It fills the
//!   radio card and the map legend today, and returns `None` for the rest.
//! - [`modded`]: the themes the mods register through `bb.ui`. A mod theme is selected with [`UiThemes::select_mod`], by the
//!   Mods page's theme choice through [`UiThemes::set_mod_choice`]; the style setting does not choose it
//!   (`docs/specs/ui-themes.md` section 12).

mod compose;
#[cfg(test)]
mod compose_tests;
pub mod custom;
mod element;
mod lan;
mod lan_hud;
#[cfg(test)]
mod lan_tests;
mod menu_list;
pub mod modded;
pub mod original;
pub mod remastered;
#[cfg(test)]
mod tests;

pub use compose::{hud_layer, slot_layer};

pub use element::ElementId;
pub use lan::{LanCell, LanHud, LanLine, LanLineKind, LanPage, LanSpectator};
pub use menu_list::{
    DEFAULT_ASPECT, MenuEntry, MenuHint, MenuIcon, MenuList, MenuListCell, MenuNudge, MenuStyle, MenuValue, RivalInfo,
    RivalStatus,
};

use bevy_ecs::prelude::*;
use blackbox_ui::{BuildCx, Component, DrawList, LayoutCx, Rect, Skin, Theme, ThemeError, ThemeId, Themes};

use crate::frontend::world_map::Chrome;
use crate::settings::{Settings, UiStyle, UiTheme};
use custom::tokens::{self as custom_tokens, CustomLook};
use custom::{CustomFeedCell, CustomTheme, DriveFeedCell, RadioFeedCell, WorldMapLegendCell, palette};
use modded::{ModHud, ModTheme};
use original::{FrameCell, OriginalTheme};
use remastered::{RemasteredTheme, WorldMapViewCell};

/// The theme a UI style selects. A theme that is not registered leaves Original active ([`UiThemes::select`]).
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
    themes.bypass_change_detection().set_ui_theme(settings.ui_theme);
    themes.bypass_change_detection().set_custom_look(CustomLook::of(&settings.hud_options));
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

/// Builds the registry: Original as the floor, the Custom and Remastered themes registered beside it, Original active.
/// The Original theme reads its trees from `frame`, Remastered its drive readouts from `drive`, and Custom its HUD from
/// `custom` (both their radio card from `radio`). The caller keeps a clone of each so it can fill them each frame.
///
/// Panics when the Original theme leaves a catalogue slot unfilled. That is a programming error, found at start-up.
/// The generic `Themes::new` cannot make this check, because it does not know the MW catalogue
/// (`docs/specs/ui-themes.md` section 7).
pub fn build_themes(
    frame: FrameCell,
    radio: RadioFeedCell,
    drive: DriveFeedCell,
    custom: CustomFeedCell,
    legend: WorldMapLegendCell,
    menus: MenuListCell,
    map: WorldMapViewCell,
) -> Themes {
    check_floor_fills_every_slot(&frame);
    let mut themes =
        Themes::new(Box::new(OriginalTheme::new(frame.clone()))).expect("OriginalTheme reports ThemeId::Original");
    themes
        .register(Box::new(CustomTheme::new(radio.clone(), custom, legend)))
        .expect("the Bevy theme is registered once, and is not Original");
    themes
        .register(Box::new(RemasteredTheme::new(frame.clone(), drive, radio, menus, map)))
        .expect("RemasteredTheme is registered once, and is not Original");
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

/// The base skin the themes hand to their components: the Custom card's and legend's fonts (T9), and the Remastered
/// dial's and readout's. The keys are namespaced by theme and element, so they do not clash. The active theme's tokens
/// and the `custom.*` tokens are merged into it ([`UiThemes::refresh_skin`]).
pub fn default_skin() -> Skin {
    custom::card_skin().merged(remastered::skin()).merged(original::lan::skin())
}

/// The theme registry, the skin and the cells, as one Bevy resource. The HUD and the menus resolve their slots through it
/// each frame (the composer, `compose.rs`).
#[derive(Resource)]
pub struct UiThemes {
    themes: Themes,
    /// The skin without any theme's tokens (see [`default_skin`]).
    base: Skin,
    /// `base` with the active theme's tokens merged in: what the components read. Rebuilt by [`UiThemes::refresh_skin`].
    skin: Skin,
    /// The theme, palette and Custom look `skin` was built for, or `None` when it must be built again.
    skin_of: Option<(ThemeId, UiTheme, CustomLook)>,
    /// The palette the token-driven HUD takes its colours from (the settings' `ui_theme`).
    ui_theme: UiTheme,
    /// The Custom HUD's colour-vision palette and high contrast (the settings' `[custom_hud]`).
    custom_look: CustomLook,
    frame: FrameCell,
    radio: RadioFeedCell,
    drive: DriveFeedCell,
    /// The frame the Custom HUD draws from. Filled by the HUD plugin each frame.
    custom: CustomFeedCell,
    /// The open world map's legend, read by the Remastered map legend. Filled by the frontend plugin while the map is
    /// open under a theme that recolours it, cleared otherwise.
    legend: WorldMapLegendCell,
    /// The icon menus' lists, read by the Remastered vertical menus. Filled by the frontend plugin each frame.
    menus: MenuListCell,
    /// The open map as the Remastered theme draws it. Filled by the frontend plugin while the map is open.
    map: WorldMapViewCell,
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
        let drive = DriveFeedCell::new();
        let custom = CustomFeedCell::new();
        let legend = WorldMapLegendCell::new();
        let menus = MenuListCell::new();
        let map = WorldMapViewCell::new();
        let mut registry = Self {
            themes: build_themes(
                frame.clone(),
                radio.clone(),
                drive.clone(),
                custom.clone(),
                legend.clone(),
                menus.clone(),
                map.clone(),
            ),
            base: default_skin(),
            skin: default_skin(),
            skin_of: None,
            ui_theme: UiTheme::default(),
            custom_look: CustomLook::default(),
            frame,
            radio,
            drive,
            custom,
            legend,
            menus,
            map,
            style: UiStyle::Original,
            mod_choice: None,
            mods: Vec::new(),
        };
        registry.apply();
        registry
    }

    /// Makes the theme for `style` active, unless a mod theme was chosen. A theme that is not registered leaves Original
    /// active, so the registry never names a theme that nothing can fill.
    pub fn select(&mut self, style: UiStyle) {
        self.style = style;
        self.apply();
    }

    /// Sets the palette the token-driven HUD draws in, once per frame. A change rebuilds the skin.
    pub fn set_ui_theme(&mut self, theme: UiTheme) {
        self.ui_theme = theme;
        self.refresh_skin();
    }

    /// Sets the Custom HUD's colour look (`[custom_hud] palette` and `high_contrast`), once per frame. A change rebuilds
    /// the skin.
    pub fn set_custom_look(&mut self, look: CustomLook) {
        self.custom_look = look;
        self.refresh_skin();
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
        self.themes = build_themes(
            self.frame.clone(),
            self.radio.clone(),
            self.drive.clone(),
            self.custom.clone(),
            self.legend.clone(),
            self.menus.clone(),
            self.map.clone(),
        );
        for theme in &self.mods {
            let hud = ModHud::new(theme.clone(), self.radio.clone(), self.custom.clone());
            // The port gives each mod theme its own id, so a refusal here is a bug. It is logged and the theme skipped.
            if let Err(error) = self.themes.register(Box::new(hud)) {
                log::warn!("the mod theme {:?} is not registered: {error}", theme.id());
            }
        }
        // A theme keeps its id when its tokens change, so the skin is built again even for the same active theme.
        self.skin_of = None;
        self.apply();
    }

    /// The active theme is the chosen mod theme, else the one the style names; a theme that is not registered leaves
    /// Original active.
    fn apply(&mut self) {
        let wanted = self.mod_choice.unwrap_or(theme_of(self.style));
        if self.themes.set_active(wanted).is_err() {
            self.themes.set_active(ThemeId::Original).expect("the Original theme is always registered");
        }
        self.refresh_skin();
    }

    /// Builds the skin for the active theme: the base skin with its tokens merged in, then the `custom.*` tokens of the
    /// Custom look, whose accent is the active tokens' `secondary`. A skin already built for the active theme, palette and
    /// look is kept, so the per-frame [`UiThemes::select`] costs one comparison.
    fn refresh_skin(&mut self) {
        let active = self.themes.active();
        let wanted = (active, self.ui_theme, self.custom_look);
        if self.skin_of == Some(wanted) {
            return;
        }
        // The token-driven HUD and the Remastered HUD take the palette the settings chose; every other theme brings its
        // own tokens.
        let tokens = match active {
            ThemeId::Custom | ThemeId::Remastered => palette::for_ui_theme(self.ui_theme),
            _ => self.themes.active_tokens(),
        };
        self.skin = self
            .base
            .clone()
            .merged(tokens.skin())
            .merged(remastered::palette_skin(&tokens))
            .merged(custom_tokens::skin(self.custom_look, tokens.secondary));
        self.skin_of = Some(wanted);
    }

    /// The skin the active theme's components read: the base skin with the active tokens merged in.
    pub fn skin(&self) -> &Skin {
        &self.skin
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

    /// Whether the HUD layers are a function of the frame cell alone: the Original theme fills every slot with an adapter
    /// that copies nodes out of the cell, whereas the other themes draw from live feeds and the clock. A layer composed
    /// while this holds is still the layer while the cell has not changed (`FrameCell::generation`).
    pub fn hud_is_static(&self) -> bool {
        self.themes.active() == ThemeId::Original
    }

    /// Whether a menu slot's layer is a function of the menu cell alone (the Original adapters copy the screen's tree out
    /// of the cell; the Remastered menus draw from lists and the clock, the other themes from their own feeds). A layer
    /// composed while this holds is still the layer while the slot's tree is the same one.
    pub fn menus_are_static(&self) -> bool {
        self.themes.active() == ThemeId::Original
    }

    /// How the active theme dresses the world map: Original leaves the FEng screen alone, Remastered draws the whole map
    /// itself, and any other theme that provides the slot (Custom) recolours the FEng screen in place and draws a legend
    /// over it.
    pub fn world_map_chrome(&self) -> Chrome {
        match self.provided_by(ElementId::WorldMapChrome) {
            None | Some(ThemeId::Original) => Chrome::Original,
            Some(ThemeId::Remastered) => Chrome::Drawn,
            Some(_) => Chrome::Recoloured,
        }
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

    /// The drive feed the Remastered cluster reads. The HUD plugin keeps this clone and calls `set` on it once per frame,
    /// after the HUD state is updated.
    pub fn drive(&self) -> DriveFeedCell {
        self.drive.clone()
    }

    /// The feed the Custom HUD reads. The HUD plugin calls `set` on it once per frame, next to the drive feed.
    pub fn custom(&self) -> CustomFeedCell {
        self.custom.clone()
    }

    /// The lists the Remastered vertical menus draw. The frontend plugin keeps this clone, clears it and sets each screen's
    /// list once per frame.
    pub fn menu_lists(&self) -> MenuListCell {
        self.menus.clone()
    }

    /// The map the Remastered theme draws. The frontend plugin keeps this clone and calls `set` each frame the map is open,
    /// and `clear` when it is closed.
    pub fn world_map_view(&self) -> WorldMapViewCell {
        self.map.clone()
    }

    /// The legend the Custom map draws. The frontend plugin keeps this clone and calls `set` while the map is open
    /// under a theme that recolours it, or `clear` otherwise.
    pub fn world_map_legend(&self) -> WorldMapLegendCell {
        self.legend.clone()
    }
}
