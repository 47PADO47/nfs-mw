//! The Custom skin: the fonts and the status colours the Custom components read, by key. The colours a theme shows its
//! panels and text in are not here: they are the theme's tokens (`blackbox_ui::Tokens`), which the registry writes into
//! the skin of the active theme. `ui/theme/mod.rs` hands the base skin to every component, so the keys are namespaced
//! (`radio.*`, `drive.*`); the Original components never read them.

use blackbox_feng::{fe_hash_upper, resource_handle};
use blackbox_ui::Skin;

use crate::frontend::world_map::control::MAP_ICON_FILES;
use crate::ui::raster_text::{RASTER_MONO_FONT_KEY, RASTER_UPRIGHT_FONT_KEY};

/// The skin keys the radio card reads. Its colours are tokens; only the font is the card's own.
pub(super) mod keys {
    use blackbox_ui::token_keys;

    pub const BACKGROUND: &str = token_keys::BACKGROUND;
    pub const TITLE: &str = token_keys::PRIMARY;
    pub const CREDIT: &str = token_keys::MUTED;
    pub const STATUS: &str = token_keys::MUTED;
    pub const FONT: &str = "radio.font";
    /// The map legend's font: the upright monospace the FPS overlay uses, so the legend is not leaned like the cluster.
    pub const LEGEND_FONT: &str = "legend.font";
}

/// The skin keys the drive cluster reads: the card, the rev gauge, the gear, and the three meters. The card's panel is
/// the token background; the rest of the cluster's colours are its own (the cyan and amber of the rev gauge, the red of
/// the redline and the shift light, and one colour per meter).
pub(super) mod drive {
    use blackbox_ui::token_keys;

    pub const BACKGROUND: &str = token_keys::BACKGROUND;
    pub const VALUE: &str = token_keys::PRIMARY;
    pub const LABEL: &str = token_keys::MUTED;
    pub const UNIT: &str = token_keys::MUTED;
    pub const REV_LIT: &str = token_keys::SECONDARY;
    pub const REV_WARN: &str = "drive.rev.warn";
    pub const REV_REDLINE: &str = "drive.rev.redline";
    pub const TRACK: &str = token_keys::BORDER;
    pub const GEAR_BOX: &str = token_keys::BORDER;
    pub const GEAR_SHIFT: &str = "drive.gear.shift";
    pub const GEAR_LETTER: &str = token_keys::SECONDARY;
    pub const PIP_OFF: &str = token_keys::BORDER;
    pub const PIP_ON: &str = token_keys::SECONDARY;
    pub const NOS: &str = token_keys::SECONDARY;
    pub const BOOST: &str = token_keys::SECONDARY;
    pub const SPEEDBREAKER: &str = token_keys::SECONDARY;
    pub const FONT: &str = "drive.font";
}

/// The Custom skin's own entries: the body font for the radio card, the raster font for the drive cluster, and the
/// fixed status colours. The theme's tokens supply every other colour, so a skin only shows the card's look once a
/// theme's tokens are merged into it.
pub fn card_skin() -> Skin {
    let body = fe_hash_upper("FONT_MW_BODY");
    let mut skin = Skin::new()
        .with_font(keys::FONT, body)
        .with_font(keys::LEGEND_FONT, RASTER_MONO_FONT_KEY)
        .with_colour(drive::REV_WARN, [255, 190, 60, 255])
        .with_colour(drive::REV_REDLINE, [255, 80, 90, 255])
        .with_colour(drive::GEAR_SHIFT, [255, 80, 90, 255])
        .with_font(drive::FONT, RASTER_UPRIGHT_FONT_KEY);
    // The map's blip icons, keyed by their file name, so a map draws an icon by the file `icon_name` returns.
    for file in MAP_ICON_FILES {
        skin = skin.with_texture(file, resource_handle(file));
    }
    skin
}

/// A copy of `skin` with the card's colours' alpha multiplied by `opacity`, so the card's components draw it fading.
/// Only the card's keys are copied; the rest are left out, as the components never read them.
pub(super) fn faded(skin: &Skin, opacity: f32) -> Skin {
    let mut out = Skin::new();
    for key in [keys::BACKGROUND, keys::TITLE, keys::CREDIT, keys::STATUS] {
        if let Some(rgba) = skin.colour(key) {
            out = out.with_colour(key, fade(rgba, opacity));
        }
    }
    if let Some(font) = skin.font(keys::FONT) {
        out = out.with_font(keys::FONT, font);
    }
    out
}

fn fade([r, g, b, a]: [u8; 4], opacity: f32) -> [u8; 4] {
    [r, g, b, (f32::from(a) * opacity).round() as u8]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::custom::palette;
    use blackbox_ui::token_keys;

    /// The skin a theme's components see: the Custom skin with the Bevy tokens merged in.
    fn bevy_skin() -> Skin {
        card_skin().merged(palette::bevy().skin())
    }

    #[test]
    fn the_font_is_the_prompt_body_font() {
        // `frontend/prompts.rs` names FONT_MW_BODY by its hash; the card must read the same font.
        assert_eq!(card_skin().font(keys::FONT), Some(0x5455_70c6));
    }

    #[test]
    fn fading_scales_only_the_alpha_of_the_card_keys() {
        let skin = bevy_skin();
        let faded = faded(&skin, 0.5);
        assert_eq!(faded.colour(keys::BACKGROUND), Some([0, 0, 0, 85]));
        assert_eq!(faded.colour(keys::TITLE), Some([255, 255, 255, 128]));
        assert_eq!(faded.font(keys::FONT), skin.font(keys::FONT));
        assert_eq!(faded.colour("text"), None);
    }

    #[test]
    fn the_card_takes_its_colours_from_the_tokens() {
        let skin = bevy_skin();
        assert_eq!(skin.colour(token_keys::BACKGROUND), Some(palette::bevy().background));
        assert_eq!(skin.colour(token_keys::PRIMARY), Some(palette::bevy().primary));
    }
}
