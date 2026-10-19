//! The devtools' look, taken from the active theme: its palette, corner radius, padding and text size.
//!
//! The F12 console and the panels are egui views. They follow the theme the options menu chose by reading the same
//! tokens the game's components read, so a theme change reaches them on the next frame. The font is the one exception:
//! the game's fonts are FEng bitmaps, not the TTF files egui loads, so the panels keep egui's font.

use blackbox_ui::{Skin, token_keys};
use egui::{Color32, CornerRadius, FontId, Margin, Stroke, Style, Visuals};

/// The window fill the devtools used before they followed a theme: a dark panel behind the text.
const FALLBACK_FILL: Color32 = Color32::from_black_alpha(170);

/// The egui style the devtools draw with, built from the active theme's skin.
pub fn style_from(skin: &Skin) -> Style {
    let mut style = Style::default();
    let visuals = &mut style.visuals;
    let primary = colour(skin, token_keys::PRIMARY);
    let secondary = colour(skin, token_keys::SECONDARY);
    let border = colour(skin, token_keys::BORDER);
    let radius = metric(skin, token_keys::RADIUS).map_or(0, |r| r.round().clamp(0.0, 255.0) as u8);
    apply_palette(visuals, skin, primary, secondary, border);
    apply_radius(visuals, radius);
    if let Some(padding) = metric(skin, token_keys::PADDING) {
        let pad = padding.round().clamp(0.0, 127.0) as i8;
        style.spacing.window_margin = Margin::same(pad);
        style.spacing.button_padding = egui::vec2(f32::from(pad), f32::from(pad) / 2.0);
    }
    if let Some(scale) = metric(skin, token_keys::TEXT_SCALE) {
        scale_text(&mut style, scale.max(0.5));
    }
    style
}

fn apply_palette(
    visuals: &mut Visuals,
    skin: &Skin,
    primary: Option<Color32>,
    secondary: Option<Color32>,
    border: Option<Color32>,
) {
    if let Some(text) = primary {
        visuals.override_text_color = Some(text);
    }
    if let Some(accent) = secondary {
        visuals.hyperlink_color = accent;
        visuals.selection.bg_fill = accent.gamma_multiply(0.45);
    }
    if let Some(rule) = border {
        visuals.window_stroke = Stroke::new(1.0, rule);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, rule);
    }
    // A transparent background token means "no fill of its own": keep the devtools' dark panel.
    let background = colour(skin, token_keys::BACKGROUND).filter(|c| c.a() > 0);
    let fill = background.unwrap_or(FALLBACK_FILL);
    visuals.window_fill = fill;
    visuals.panel_fill = fill;
}

fn apply_radius(visuals: &mut Visuals, radius: u8) {
    let corners = CornerRadius::same(radius);
    visuals.window_corner_radius = corners;
    visuals.menu_corner_radius = corners;
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = corners;
    }
}

/// Scales every text style's size by the theme's `text_scale`, so the theme's text size reaches the panels.
fn scale_text(style: &mut Style, scale: f32) {
    for font in style.text_styles.values_mut() {
        *font = FontId::new(font.size * scale, font.family.clone());
    }
}

fn colour(skin: &Skin, key: &str) -> Option<Color32> {
    let [r, g, b, a] = skin.colour(key)?;
    Some(Color32::from_rgba_unmultiplied(r, g, b, a))
}

fn metric(skin: &Skin, key: &str) -> Option<f32> {
    skin.metric(key).filter(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_ui::Tokens;
    use egui::TextStyle;

    fn skin_with(tokens: Tokens) -> Skin {
        tokens.skin()
    }

    #[test]
    fn the_palette_and_shape_tokens_reach_the_style() {
        let tokens = Tokens {
            primary: [10, 20, 30, 255],
            secondary: [200, 100, 50, 255],
            background: [40, 40, 40, 255],
            border: [90, 90, 90, 255],
            radius: 6.0,
            padding: 8.0,
            ..Tokens::default()
        };
        let style = style_from(&skin_with(tokens));
        assert_eq!(style.visuals.override_text_color, Some(Color32::from_rgb(10, 20, 30)));
        assert_eq!(style.visuals.window_fill, Color32::from_rgb(40, 40, 40));
        assert_eq!(style.visuals.window_corner_radius, CornerRadius::same(6));
        assert_eq!(style.spacing.window_margin, Margin::same(8));
    }

    #[test]
    fn a_transparent_background_keeps_the_devtools_panel_fill() {
        let tokens = Tokens { background: [0, 0, 0, 0], ..Tokens::default() };
        assert_eq!(style_from(&skin_with(tokens)).visuals.window_fill, FALLBACK_FILL);
    }

    #[test]
    fn text_scale_resizes_every_text_style() {
        let base = Style::default();
        let tokens = Tokens { text_scale: 2.0, ..Tokens::default() };
        let style = style_from(&skin_with(tokens));
        let before = base.text_styles[&TextStyle::Body].size;
        assert_eq!(style.text_styles[&TextStyle::Body].size, before * 2.0);
    }
}
