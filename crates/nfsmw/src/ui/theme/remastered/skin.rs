//! The Remastered skin: the colours and fonts the cluster reads, by key. Keys are namespaced `remastered.*`, so they sit
//! beside the Custom keys in the one skin the registry hands out (`ui/theme/mod.rs`).

use crate::ui::raster_text::RASTER_FONT_KEY;
use blackbox_ui::Skin;

/// The skin keys the cluster reads.
pub(super) mod keys {
    pub const ARC_LIT: &str = "remastered.arc.lit";
    pub const ARC_DIM: &str = "remastered.arc.dim";
    pub const ARC_REDLINE: &str = "remastered.arc.redline";
    pub const ARC_REDLINE_DIM: &str = "remastered.arc.redline_dim";
    pub const TICK: &str = "remastered.tick";
    pub const TICK_REDLINE: &str = "remastered.tick.redline";
    pub const VALUE: &str = "remastered.value";
    pub const UNIT: &str = "remastered.unit";
    pub const UNIT_HIGHLIGHT: &str = "remastered.unit.highlight";
    pub const GEAR: &str = "remastered.gear";
    pub const BAR_TRACK: &str = "remastered.bar.track";
    pub const BAR_FILL: &str = "remastered.bar.fill";
    pub const BAR_LABEL: &str = "remastered.bar.label";
    pub const FONT_NUMBERS: &str = "remastered.font.numbers";
    pub const FONT_TEXT: &str = "remastered.font.text";
}

/// The Remastered skin: white arc and numbers, gold for the red line and the highlighted H of MPH, a grey unit, and a solid
/// gold bar. Every text uses the same font (`raster_text`).
pub fn skin() -> Skin {
    Skin::new()
        .with_colour(keys::ARC_LIT, [255, 255, 255, 255])
        .with_colour(keys::ARC_DIM, [255, 255, 255, 120])
        .with_colour(keys::ARC_REDLINE, [255, 196, 64, 255])
        .with_colour(keys::ARC_REDLINE_DIM, [255, 196, 64, 120])
        .with_colour(keys::TICK, [235, 235, 235, 255])
        .with_colour(keys::TICK_REDLINE, [255, 196, 64, 255])
        .with_colour(keys::VALUE, [255, 255, 255, 255])
        .with_colour(keys::UNIT, [170, 170, 170, 255])
        .with_colour(keys::UNIT_HIGHLIGHT, [255, 196, 64, 255])
        .with_colour(keys::GEAR, [255, 255, 255, 255])
        .with_colour(keys::BAR_TRACK, [255, 255, 255, 34])
        .with_colour(keys::BAR_FILL, [255, 190, 70, 255])
        .with_colour(keys::BAR_LABEL, [255, 196, 64, 255])
        .with_font(keys::FONT_NUMBERS, RASTER_FONT_KEY)
        .with_font(keys::FONT_TEXT, RASTER_FONT_KEY)
}
