//! The Remastered skin: maps the cluster's keys to the theme's token colours where they express the same intent (primary
//! for bright digits, secondary for gold accents, muted for dim labels, border for tracks). Keys that are tints of a
//! token colour stay as custom entries. Font scale metrics let the components read their sizes from the skin.

use blackbox_feng::resource_handle;
use blackbox_ui::{Skin, Tokens};

use crate::ui::input_icons;
use crate::ui::raster_text::{
    RASTER_FONT_KEY, RASTER_MENU_FONT_KEY, RASTER_MENU_TITLE_FONT_KEY, RASTER_MENU_WIDE_FONT_KEY,
    RASTER_NUMBER_FONT_KEY, RASTER_TIMER_FONT_KEY,
};

/// The skin keys the cluster reads. Token-backed keys point directly at `token_keys::*`; custom keys are namespaced
/// `remastered.*` so they sit beside the Custom theme's keys in the shared skin.
pub(super) mod keys {
    use blackbox_ui::token_keys;

    pub const ARC_LIT: &str = token_keys::PRIMARY;
    pub const ARC_DIM: &str = token_keys::BORDER;
    pub const ARC_REDLINE: &str = token_keys::SECONDARY;
    pub const ARC_REDLINE_DIM: &str = "remastered.arc.redline_dim";
    pub const TICK: &str = token_keys::MUTED;
    pub const TICK_REDLINE: &str = token_keys::SECONDARY;
    pub const VALUE: &str = token_keys::PRIMARY;
    pub const UNIT: &str = token_keys::MUTED;
    pub const UNIT_HIGHLIGHT: &str = token_keys::SECONDARY;
    pub const GEAR: &str = token_keys::PRIMARY;
    /// The lighter panel behind the gear digit.
    pub const GEAR_PANEL: &str = "remastered.gear.panel";
    /// The dark band the gauges stand on, under the card.
    pub const GEAR_BAND: &str = "remastered.gear.band";
    pub const BAR_TRACK: &str = token_keys::BORDER;
    pub const BAR_FILL: &str = token_keys::SECONDARY;
    pub const BAR_BORDER: &str = token_keys::BORDER;
    pub const CLUSTER_BG: &str = token_keys::BACKGROUND;
    pub const FONT_NUMBERS: &str = "remastered.font.numbers";
    pub const FONT_TEXT: &str = "remastered.font.text";
    pub const SCALE_SPEED: &str = "remastered.scale.speed";
    pub const SCALE_UNIT: &str = "remastered.scale.unit";
    pub const SCALE_GEAR: &str = "remastered.scale.gear";
    pub const SCALE_TICK: &str = "remastered.scale.tick";
    pub const SCALE_BAR_LABEL: &str = "remastered.scale.bar_label";
    /// The vertical menus (`menu/`; their colours follow the selected palette, see [`palette_skin`]): the highlight bar,
    /// the entries' text in its three states, the dark shade behind the left of the screen, the footer's rules, and the
    /// lightning bolt at the selected entry.
    pub const MENU_BAR: &str = "remastered.menu.bar";
    pub const MENU_TEXT: &str = "remastered.menu.text";
    pub const MENU_TEXT_SELECTED: &str = "remastered.menu.text_selected";
    pub const MENU_TEXT_DISABLED: &str = "remastered.menu.text_disabled";
    pub const MENU_SHADE: &str = "remastered.menu.shade";
    /// The dim over the frozen world behind an in-game menu, the dark inside a slider's track, and the double arrow.
    pub const MENU_SCRIM: &str = "remastered.menu.scrim";
    pub const MENU_TRACK: &str = "remastered.menu.track";
    pub const MENU_ARROW: &str = "remastered.menu.arrow";
    pub const MENU_RULE: &str = "remastered.menu.rule";
    pub const MENU_BOLT: &str = "remastered.menu.bolt";
    pub const MENU_TAGLINE: &str = "remastered.menu.tagline";
    pub const FONT_MENU: &str = "remastered.font.menu";
    pub const FONT_MENU_WIDE: &str = "remastered.font.menu_wide";
    pub const FONT_MENU_TITLE: &str = "remastered.font.menu_title";
    pub const FONT_TIMER: &str = "remastered.font.timer";
    /// The full-screen map: the ground behind the picture, the dark band under its footer and the plate behind a label.
    pub const MAP_GROUND: &str = "remastered.map.ground";
    pub const MAP_BAND: &str = "remastered.map.band";
    pub const MAP_VEIL: &str = "remastered.map.veil";
    pub const MAP_TIP: &str = "remastered.map.tip";
    /// The menu title's colour: the palette's accent.
    pub const MENU_TITLE: &str = "remastered.menu.title";
    /// The game's logo, and the controller glyph atlas the footer hints draw from.
    pub const LOGO: &str = "remastered.menu.logo";
    pub const ATLAS: &str = "remastered.menu.atlas";
}

/// The Remastered entries that follow the selected palette: the redline tint of its secondary colour, the menus' bar,
/// text and rules, and the font sizes, which the palette's `text_scale` multiplies. The registry merges it over the palette's tokens for the Remastered HUD.
pub fn palette_skin(tokens: &Tokens) -> Skin {
    let [r, g, b, _] = tokens.secondary;
    let scale = tokens.text_scale;
    Skin::new()
        .with_colour(keys::ARC_REDLINE_DIM, [r, g, b, 100])
        .with_colour(keys::GEAR_PANEL, [tokens.primary[0], tokens.primary[1], tokens.primary[2], 20])
        .with_metric(keys::SCALE_SPEED, 3.25 * scale)
        .with_metric(keys::SCALE_UNIT, 1.15 * scale)
        .with_metric(keys::SCALE_GEAR, 2.9 * scale)
        .with_metric(keys::SCALE_TICK, 1.0 * scale)
        .with_metric(keys::SCALE_BAR_LABEL, 0.8 * scale)
        .with_colour(keys::MENU_BAR, scaled(tokens.secondary, 0.84))
        .with_colour(keys::MENU_BOLT, tokens.secondary)
        .with_colour(keys::MENU_TITLE, tokens.secondary)
        .with_colour(keys::MENU_TEXT, scaled(tokens.primary, 0.9))
        .with_colour(keys::MENU_TEXT_SELECTED, tokens.primary)
        .with_colour(keys::MENU_TEXT_DISABLED, with_alpha(tokens.primary, 95))
        .with_colour(keys::MENU_RULE, with_alpha(tokens.primary, 120))
        .with_colour(keys::MENU_TAGLINE, tokens.muted)
}

/// `colour` with its RGB scaled by `factor`.
fn scaled([r, g, b, a]: [u8; 4], factor: f32) -> [u8; 4] {
    let s = |v: u8| (f32::from(v) * factor).round().clamp(0.0, 255.0) as u8;
    [s(r), s(g), s(b), a]
}

fn with_alpha([r, g, b, _]: [u8; 4], alpha: u8) -> [u8; 4] {
    [r, g, b, alpha]
}

/// The Remastered skin. Token-backed colours are supplied by the theme's `Tokens::skin()` merge; this function provides
/// the custom entries, the fonts, the menus' logo and the controller glyph atlas.
pub fn skin() -> Skin {
    Skin::new()
        .with_font(keys::FONT_NUMBERS, RASTER_NUMBER_FONT_KEY)
        .with_font(keys::FONT_TEXT, RASTER_FONT_KEY)
        .with_font(keys::FONT_MENU, RASTER_MENU_FONT_KEY)
        .with_font(keys::FONT_MENU_WIDE, RASTER_MENU_WIDE_FONT_KEY)
        .with_font(keys::FONT_MENU_TITLE, RASTER_MENU_TITLE_FONT_KEY)
        .with_font(keys::FONT_TIMER, RASTER_TIMER_FONT_KEY)
        .with_colour(keys::MENU_SHADE, [0, 0, 0, 255])
        .with_colour(keys::GEAR_BAND, [0, 0, 0, 110])
        .with_colour(keys::MENU_SCRIM, [3, 4, 7, 150])
        .with_colour(keys::MENU_TRACK, [0, 0, 0, 130])
        .with_texture(keys::MENU_ARROW, resource_handle("ARROW_SKINNIER.tga"))
        .with_colour(keys::MAP_GROUND, [6, 7, 9, 238])
        .with_colour(keys::MAP_BAND, [0, 0, 0, 150])
        .with_colour(keys::MAP_VEIL, [4, 5, 8, 60])
        .with_colour(keys::MAP_TIP, [10, 11, 14, 225])
        .with_texture(keys::LOGO, resource_handle("MW_Logo.tga"))
        .with_texture(keys::ATLAS, input_icons::ATLAS)
}
