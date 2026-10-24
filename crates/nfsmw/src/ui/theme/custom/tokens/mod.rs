//! The Custom HUD's design tokens, in one place (ADR 0013; `docs/specs/custom-hud/visual-identity.md`): the colours as
//! skin keys under `custom.*`, the fonts, opacity, and the transition durations. No Custom component names a literal
//! colour: it names a key of [`keys`], and the skin carries the value the player's settings chose.
//!
//! - **Delivery.** [`skin`] builds the `custom.*` entries for a [`CustomLook`] (the colour-vision palette and high
//!   contrast) and an accent colour. `UiThemes::refresh_skin` merges them into the skin every component reads, next to
//!   the `theme.*` tokens, and rebuilds it only when the look, the theme or `ui_theme` changes.
//! - **Accent.** The accent is the active theme's `secondary` token, so `ui_theme` keeps choosing the accent family (the
//!   Bevy cyan or the Remastered gold, `palette.rs`), and a mod theme with tokens brings its own. The meaning colours
//!   (`nav`, `success`, `caution`, `danger`, `critical`) keep their meaning in either family.
//! - **Opacity** is not in the skin: a component multiplies its colours' alpha by `[custom_hud] opacity` (and its own
//!   fade) when it lays out, with [`fade`] and [`faded_skin`]. A safety message never drops below [`SAFETY_OPACITY`].
//! - **Scale and text size** are the layout's (`layout.rs`) and the text helpers' (`text.rs`).
//!
//! Submodules:
//!
//! - [`colours`]: the brief's palette, high contrast and the colour-vision palettes.
//! - [`motion`]: the transition durations and motion reduction.

mod colours;
mod motion;
#[cfg(test)]
mod tests;

pub use colours::Colours;
#[allow(unused_imports)] // the HUD components of the later PRs read the motion table
pub use motion::{Motion, ramp};

use blackbox_ui::Skin;

use crate::settings::hud::{HudPalette, HudSettings};
use crate::ui::raster_text::{CUSTOM_LABEL_FONT_KEY, CUSTOM_LABEL_WIDE_FONT_KEY, CUSTOM_NUMERIC_FONT_KEY};

/// The skin keys of the Custom tokens. Components read these, never a palette.
pub mod keys {
    /// Digits and primary labels.
    pub const TEXT: &str = "custom.text";
    /// Units, secondary labels, inactive rows.
    pub const TEXT_SECONDARY: &str = "custom.text_secondary";
    /// The charcoal plate behind text.
    pub const SURFACE: &str = "custom.surface";
    /// A selected menu row, an expanded chip.
    pub const SURFACE_RAISED: &str = "custom.surface_raised";
    /// Tracks, empty segments, separators.
    pub const RULE: &str = "custom.rule";
    /// The theme accent (`ui_theme`): the gear box edge, a selected row's bar, a plate's top line.
    pub const ACCENT: &str = "custom.accent";
    /// Navigation: the route, the active checkpoint, the turn cue.
    pub const NAV: &str = "custom.nav";
    /// Escaped, AHEAD, a checkpoint confirmed.
    pub const SUCCESS: &str = "custom.success";
    /// The rev strip near the shift point, a degraded state.
    pub const CAUTION: &str = "custom.caution";
    /// Pursuit, the red line, BEHIND.
    pub const DANGER: &str = "custom.danger";
    /// A critical warning's solid plate, and the dark text on it.
    pub const CRITICAL: &str = "custom.critical";
    pub const CRITICAL_TEXT: &str = "custom.critical_text";
    /// The one-unit outline round every plate. Present in the skin only with high contrast on.
    pub const OUTLINE: &str = "custom.outline";
    /// The three font roles (`ui/raster_text.rs`): condensed numerals, neutral labels, spaced headings.
    pub const FONT_NUMERIC: &str = "custom.font.numeric";
    pub const FONT_LABEL: &str = "custom.font.label";
    pub const FONT_LABEL_WIDE: &str = "custom.font.label_wide";
}

/// The least alpha a safety message keeps (a critical notification, the vehicle-disabled warning), whatever
/// `[custom_hud] opacity` says.
pub const SAFETY_OPACITY: f32 = 0.8;

/// The settings that choose the Custom colours: the colour-vision palette and high contrast. Opacity is applied per
/// component, so it is not part of the look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CustomLook {
    pub palette: HudPalette,
    pub high_contrast: bool,
}

impl CustomLook {
    /// The look `[custom_hud]` chooses.
    pub fn of(settings: &HudSettings) -> Self {
        Self { palette: settings.palette, high_contrast: settings.high_contrast }
    }
}

/// The `custom.*` skin entries for `look`, with `accent` as the accent (the active theme's `secondary` token): the
/// colours, the high-contrast outline when it is on, and the three fonts.
pub fn skin(look: CustomLook, accent: [u8; 4]) -> Skin {
    let c = Colours::for_look(look);
    let skin = Skin::new()
        .with_colour(keys::TEXT, c.text)
        .with_colour(keys::TEXT_SECONDARY, c.text_secondary)
        .with_colour(keys::SURFACE, c.surface)
        .with_colour(keys::SURFACE_RAISED, c.surface_raised)
        .with_colour(keys::RULE, c.rule)
        .with_colour(keys::ACCENT, accent)
        .with_colour(keys::NAV, c.nav)
        .with_colour(keys::SUCCESS, c.success)
        .with_colour(keys::CAUTION, c.caution)
        .with_colour(keys::DANGER, c.danger)
        .with_colour(keys::CRITICAL, c.critical)
        .with_colour(keys::CRITICAL_TEXT, c.critical_text)
        .with_font(keys::FONT_NUMERIC, CUSTOM_NUMERIC_FONT_KEY)
        .with_font(keys::FONT_LABEL, CUSTOM_LABEL_FONT_KEY)
        .with_font(keys::FONT_LABEL_WIDE, CUSTOM_LABEL_WIDE_FONT_KEY);
    let Some(outline) = c.outline else { return skin };
    skin.with_colour(keys::OUTLINE, outline)
}

/// `rgba` with its alpha multiplied by `opacity` (clamped to 0 to 1).
pub fn fade([r, g, b, a]: [u8; 4], opacity: f32) -> [u8; 4] {
    let opacity = match opacity.is_nan() {
        true => 0.0,
        false => opacity.clamp(0.0, 1.0),
    };
    [r, g, b, (f32::from(a) * opacity).round() as u8]
}

/// A skin holding only `colours` (faded by `opacity`) and `fonts` from `skin`, for a component to lay out a part at an
/// alpha. A key `skin` lacks is left out, so a component reading it falls back as it would on `skin`.
pub fn faded_skin(skin: &Skin, colours: &[&str], fonts: &[&str], opacity: f32) -> Skin {
    let mut out = Skin::new();
    for key in colours {
        if let Some(rgba) = skin.colour(key) {
            out = out.with_colour(*key, fade(rgba, opacity));
        }
    }
    for key in fonts {
        if let Some(font) = skin.font(key) {
            out = out.with_font(*key, font);
        }
    }
    out
}

/// The opacity a safety message draws at: the HUD's `opacity`, but never below [`SAFETY_OPACITY`].
pub fn safety_opacity(opacity: f32) -> f32 {
    opacity.max(SAFETY_OPACITY)
}
