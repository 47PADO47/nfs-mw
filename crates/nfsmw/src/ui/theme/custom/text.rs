//! Text in the Custom HUD's three font roles (`docs/specs/custom-hud/typography.md`): condensed numerals with tabular
//! digits, neutral labels, and spaced headings. A [`Label`] names its role, its size in HUD units at scale 1 and a
//! token colour; [`Label::laid_out`] turns it into a `blackbox_ui::Text` sized by the HUD scale and `[custom_hud]
//! text_size`, with its colour faded by the opacity.
//!
//! Numbers keep a stable width because the numeric role's font key (`CUSTOM_NUMERIC_FONT_KEY`) is drawn with tabular
//! digits by the raster text (`ui/raster_text.rs`), whatever the face.

use blackbox_ui::{Component, DEFAULT_Z, LayoutCx, Rect, Text};

use super::layout::HudBox;
use super::tokens::{faded_skin, keys};
use crate::settings::hud::HudSettings;
use crate::ui::raster_text::UNITS_PER_SCALE;

/// The sizes of the type hierarchy at scale 1, in HUD units (the height of the text).
pub mod size {
    pub const SPEED: f32 = 44.0;
    pub const GEAR: f32 = 26.0;
    pub const POSITION: f32 = 30.0;
    /// The race strip's values: lap, distance, gap.
    pub const STRIP_VALUE: f32 = 16.0;
    /// The speed unit and short headings (`POS`, `LAP`, `HEAT`).
    pub const HEADING: f32 = 10.0;
    pub const NOTIFICATION_TITLE: f32 = 14.0;
    /// A notification's second line, a chip's text.
    pub const NOTIFICATION_LINE: f32 = 11.0;
    pub const MENU_ROW: f32 = 16.0;
    pub const MENU_TITLE: f32 = 22.0;
}

/// The three font roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Condensed, semi-bold, tabular digits: speed, gear, places, counts, timers, distances, gaps.
    Numeric,
    /// Neutral regular: notification text, secondary labels, menu rows.
    Label,
    /// Regular width, semi-bold, spaced: short uppercase headings.
    LabelWide,
}

impl Role {
    /// The skin font key of the role (`custom.font.*`).
    pub fn font_key(self) -> &'static str {
        match self {
            Role::Numeric => keys::FONT_NUMERIC,
            Role::Label => keys::FONT_LABEL,
            Role::LabelWide => keys::FONT_LABEL_WIDE,
        }
    }
}

/// How large the HUD's text is: the effective HUD scale (`HudBox::scale`) and `[custom_hud] text_size`, which scales
/// text only, on top of the scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sizing {
    pub scale: f32,
    pub text_size: f32,
}

impl Sizing {
    pub fn new(scale: f32, text_size: f32) -> Self {
        Self { scale, text_size }
    }

    /// The sizing of `frame`'s scale and `settings`' text size.
    pub fn of(frame: &HudBox, settings: &HudSettings) -> Self {
        Self::new(frame.scale(), settings.text_size)
    }

    /// The drawn height of text `size` units high at scale 1, in canvas units.
    pub fn units(self, size: f32) -> f32 {
        size * self.scale * self.text_size
    }

    /// The `Text` glyph scale that draws text `size` units high at scale 1: the raster text is `UNITS_PER_SCALE` units
    /// high at glyph scale 1.
    pub fn glyph_scale(self, size: f32) -> f32 {
        self.units(size) / UNITS_PER_SCALE
    }
}

impl Default for Sizing {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}

/// A string to draw in one role, size and token colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    role: Role,
    text: String,
    size: f32,
    colour: &'static str,
    justification: u32,
    opacity: f32,
    z: f32,
}

impl Label {
    /// `text` in `role`, `size` units high at scale 1, in the primary text colour.
    pub fn new(role: Role, text: impl Into<String>, size: f32) -> Self {
        Self { role, text: text.into(), size, colour: keys::TEXT, justification: 0, opacity: 1.0, z: DEFAULT_Z }
    }

    /// A number in the condensed numeric face (tabular digits), in the primary text colour.
    pub fn numeric(text: impl Into<String>, size: f32) -> Self {
        Self::new(Role::Numeric, text, size)
    }

    /// A label in the neutral face, in the primary text colour.
    #[allow(clippy::self_named_constructors)] // `Label::label` reads as the role it builds, beside `numeric` and `heading`
    pub fn label(text: impl Into<String>, size: f32) -> Self {
        Self::new(Role::Label, text, size)
    }

    /// A short heading in the spaced face, in the secondary text colour.
    pub fn heading(text: impl Into<String>, size: f32) -> Self {
        Self::new(Role::LabelWide, text, size).colour(keys::TEXT_SECONDARY)
    }

    /// Draws in the skin colour `key` (a `custom.*` key) instead.
    pub fn colour(self, key: &'static str) -> Self {
        Self { colour: key, ..self }
    }

    /// Sets the justification flags (`blackbox_ui::justify`).
    pub fn justify(self, flags: u32) -> Self {
        Self { justification: flags, ..self }
    }

    /// Multiplies the colour's alpha by `opacity`: `[custom_hud] opacity` times the element's own fade.
    pub fn opacity(self, opacity: f32) -> Self {
        Self { opacity, ..self }
    }

    /// Sets the depth. A lower `z` draws on top.
    pub fn z(self, z: f32) -> Self {
        Self { z, ..self }
    }

    /// The `Text` that draws this label in `rect` (canvas units) at `sizing`, laid out against `cx`'s skin with the
    /// colour faded. The returned text is already laid out: build it, and do not lay it out again with an unfaded skin.
    pub fn laid_out(&self, rect: Rect, sizing: Sizing, cx: &LayoutCx<'_>) -> Text {
        let font = self.role.font_key();
        let skin = faded_skin(cx.skin, &[self.colour], &[font], self.opacity);
        let mut text = Text::new(font, self.text.clone())
            .with_colour_key(self.colour)
            .with_scale(sizing.glyph_scale(self.size))
            .with_justification(self.justification)
            .with_z(self.z);
        text.layout(&LayoutCx::new(&skin, cx.time_s), rect);
        text
    }
}
