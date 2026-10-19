//! The Remastered skin: maps the cluster's keys to the theme's token colours where they express the same intent (primary
//! for bright digits, secondary for gold accents, muted for dim labels, border for tracks). Keys that are tints of a
//! token colour stay as custom entries. Font scale metrics let the components read their sizes from the skin.

use blackbox_feng::resource_handle;
use blackbox_ui::{Skin, Tokens};

use crate::frontend::world_map::control::MAP_ICON_FILES;
use crate::ui::raster_text::{RASTER_FONT_KEY, RASTER_MONO_FONT_KEY, RASTER_NUMBER_FONT_KEY};

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
    pub const GEAR: &str = token_keys::MUTED;
    pub const BAR_TRACK: &str = token_keys::BORDER;
    pub const BAR_FILL: &str = token_keys::SECONDARY;
    pub const BAR_BORDER: &str = token_keys::BORDER;
    pub const BAR_LABEL: &str = token_keys::SECONDARY;
    pub const CLUSTER_BG: &str = token_keys::BACKGROUND;
    pub const FONT_NUMBERS: &str = "remastered.font.numbers";
    pub const FONT_TEXT: &str = "remastered.font.text";
    /// The map legend's font: the upright monospace the FPS overlay uses, so the legend is not leaned like the cluster.
    pub const FONT_LEGEND: &str = "remastered.font.legend";
    pub const SCALE_SPEED: &str = "remastered.scale.speed";
    pub const SCALE_UNIT: &str = "remastered.scale.unit";
    pub const SCALE_GEAR: &str = "remastered.scale.gear";
    pub const SCALE_TICK: &str = "remastered.scale.tick";
    pub const SCALE_BAR_LABEL: &str = "remastered.scale.bar_label";
}

/// The Remastered entries that follow the selected palette: the redline tint of its secondary colour, and the font sizes,
/// which the palette's `text_scale` multiplies. The registry merges it over the palette's tokens for the Remastered HUD.
pub fn palette_skin(tokens: &Tokens) -> Skin {
    let [r, g, b, _] = tokens.secondary;
    let scale = tokens.text_scale;
    Skin::new()
        .with_colour(keys::ARC_REDLINE_DIM, [r, g, b, 100])
        .with_metric(keys::SCALE_SPEED, 3.4 * scale)
        .with_metric(keys::SCALE_UNIT, 1.6 * scale)
        .with_metric(keys::SCALE_GEAR, 2.4 * scale)
        .with_metric(keys::SCALE_TICK, 1.4 * scale)
        .with_metric(keys::SCALE_BAR_LABEL, 0.7 * scale)
}

/// The Remastered skin. Token-backed colours are supplied by the theme's `Tokens::skin()` merge; this function provides
/// the custom entries, the fonts, and the map legend's blip-icon textures (keyed by their file name, so the legend
/// draws an icon by the file [`icon_name`](crate::frontend::world_map::control::icon_name) returns).
pub fn skin() -> Skin {
    let mut skin = Skin::new()
        .with_font(keys::FONT_NUMBERS, RASTER_NUMBER_FONT_KEY)
        .with_font(keys::FONT_TEXT, RASTER_FONT_KEY)
        .with_font(keys::FONT_LEGEND, RASTER_MONO_FONT_KEY);
    for file in MAP_ICON_FILES {
        skin = skin.with_texture(file, resource_handle(file));
    }
    skin
}
