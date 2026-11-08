//! The `[bike.dash]` table: the screen of a bike's instrument cluster, drawn live (`docs/modding/ride6-bike-dash.md`).
//!
//! A cluster's picture is a texture of the car's own `TEXTURES.BIN`. Left as the converter baked it, it shows a bike at rest. A bike
//! that has this table names that texture (stored uncompressed, so the host can write into it) and the window of it that is the
//! display; the host then paints the speed, the gear and the revs into the window a few times a second from the bike's telemetry and
//! uploads only that region. The table is optional: a bike without it keeps the picture it was baked with.
//!
//! The colours are the texel colours as they must be written (the converter's lift of the picture already applied), `[r, g, b]`.

use serde::Deserialize;

use super::ContentError;
use super::bike_check::ranged;

/// The screen of the cluster (`[bike.dash]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dash {
    /// The name of the car's texture the screen is on, as `TEXTURES.BIN` names it. It is stored as 32-bit colour.
    pub texture: String,
    /// The display in that texture: `[x, y, width, height]` in texels, from the top left.
    pub window: [u32; 4],
    /// The colours the host paints with when the cluster has no `field`.
    #[serde(default)]
    pub colors: Colors,
    /// Where the cluster's own widgets show the speed, the gear and the revs (`[[bike.dash.field]]`). With fields the host erases each
    /// rectangle with its background and draws the value in it, in the cluster's own colour; without, it paints its generic panel in
    /// `window`.
    #[serde(default)]
    pub field: Vec<Field>,
}

/// What a field shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// The speed in km/h.
    Speed,
    /// The gear: `N` in neutral, `R` in reverse.
    Gear,
    /// The revs as a bar that fills from the left.
    RpmBar,
    /// The neutral lamp: an `N` in neutral, nothing in gear.
    Neutral,
}

/// Where a text justifies in its rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Left,
    #[default]
    Center,
    Right,
}

/// One place of the cluster that shows a value (`[[bike.dash.field]]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub channel: Channel,
    /// `[x, y, width, height]` in texels of the stored texture; the whole rectangle is erased with `background` before the value is drawn.
    pub rect: [u32; 4],
    /// The colour of the digits, or of the lit part of a bar.
    pub color: [u8; 3],
    /// The colour the picture has in the rectangle without its glyphs.
    pub background: [u8; 3],
    #[serde(default)]
    pub align: Align,
    /// The height of a digit in texels (the cluster's font size); 0: as high as the rectangle allows.
    #[serde(default)]
    pub height: u32,
    /// A bar's segments (1: a continuous bar).
    #[serde(default = "one")]
    pub segments: u32,
}

fn one() -> u32 {
    1
}

/// The colours of the painted panel.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Colors {
    pub background: [u8; 3],
    /// The lit and the unlit segments of the rev bar.
    pub lit: [u8; 3],
    pub dim: [u8; 3],
    /// The speed, the unit under it and the gear.
    pub text: [u8; 3],
    pub label: [u8; 3],
    pub gear: [u8; 3],
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            background: [10, 16, 28],
            lit: [110, 200, 255],
            dim: [38, 48, 62],
            text: [235, 240, 245],
            label: [140, 150, 160],
            gear: [70, 225, 100],
        }
    }
}

impl Dash {
    /// Refuses a window outside any texture and an empty name.
    pub(super) fn check(&self) -> Result<(), ContentError> {
        ranged(self.window[2] as f32, "bike.dash.window[2]", 8.0, 8192.0)?;
        ranged(self.window[3] as f32, "bike.dash.window[3]", 8.0, 8192.0)?;
        ranged(self.window[0] as f32, "bike.dash.window[0]", 0.0, 8192.0)?;
        ranged(self.window[1] as f32, "bike.dash.window[1]", 0.0, 8192.0)?;
        for field in &self.field {
            ranged(field.rect[2] as f32, "bike.dash.field.rect[2]", 4.0, 8192.0)?;
            ranged(field.rect[3] as f32, "bike.dash.field.rect[3]", 4.0, 8192.0)?;
            ranged(field.segments as f32, "bike.dash.field.segments", 1.0, 64.0)?;
        }
        match self.texture.is_empty() || self.texture.len() > 64 {
            true => Err(super::bike_check::field("bike.dash.texture", "is empty or longer than 64 bytes")),
            false => Ok(()),
        }
    }
}
