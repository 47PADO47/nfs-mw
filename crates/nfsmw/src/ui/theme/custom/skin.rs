//! The Custom skin: the fonts and the status colours the Custom components read, by key. The colours a theme shows its
//! panels and text in are not here: they are the theme's tokens (`blackbox_ui::Tokens`), which the registry writes into
//! the skin of the active theme. `ui/theme/mod.rs` hands the base skin to every component, so the keys are namespaced
//! (`radio.*`, `drive.*`); the Original components never read them.

use blackbox_feng::fe_hash_upper;
use blackbox_ui::Skin;

use crate::ui::raster_text::RASTER_UPRIGHT_FONT_KEY;

/// The skin keys the radio card reads. Its colours are tokens; only the font is the card's own.
pub(super) mod keys {
    use blackbox_ui::token_keys;

    pub const BACKGROUND: &str = token_keys::BACKGROUND;
    pub const TITLE: &str = token_keys::PRIMARY;
    pub const CREDIT: &str = token_keys::MUTED;
    pub const STATUS: &str = token_keys::MUTED;
    pub const FONT: &str = "radio.font";
}

/// The skin keys the drive cluster reads: the card, the rev gauge, the gear, and the three meters. The card's panel is
/// the token background; the rest of the cluster's colours are its own (the cyan and amber of the rev gauge, the red of
/// the redline and the shift light, and one colour per meter).
pub(super) mod drive {
    use blackbox_ui::token_keys;

    pub const BACKGROUND: &str = token_keys::BACKGROUND;
    pub const VALUE: &str = "drive.value";
    pub const LABEL: &str = "drive.label";
    pub const UNIT: &str = "drive.unit";
    pub const REV_LIT: &str = "drive.rev.lit";
    pub const REV_WARN: &str = "drive.rev.warn";
    pub const REV_REDLINE: &str = "drive.rev.redline";
    pub const TRACK: &str = "drive.track";
    pub const GEAR_BOX: &str = "drive.gear.box";
    pub const GEAR_SHIFT: &str = "drive.gear.shift";
    pub const GEAR_LETTER: &str = "drive.gear.letter";
    pub const PIP_OFF: &str = "drive.pip.off";
    pub const PIP_ON: &str = "drive.pip.on";
    pub const NOS: &str = "drive.nos";
    pub const BOOST: &str = "drive.boost";
    pub const SPEEDBREAKER: &str = "drive.speedbreaker";
    pub const FONT: &str = "drive.font";
}

/// The Custom skin's own entries: the body font for the radio card, the raster font for the drive cluster, and the
/// fixed status colours. The theme's tokens supply every other colour, so a skin only shows the card's look once a
/// theme's tokens are merged into it.
pub fn card_skin() -> Skin {
    let body = fe_hash_upper("FONT_MW_BODY");
    Skin::new()
        .with_font(keys::FONT, body)
        .with_colour(drive::VALUE, [255, 255, 255, 255])
        .with_colour(drive::LABEL, [150, 160, 172, 255])
        .with_colour(drive::UNIT, [150, 160, 172, 255])
        .with_colour(drive::REV_LIT, [120, 220, 255, 235])
        .with_colour(drive::REV_WARN, [255, 190, 60, 255])
        .with_colour(drive::REV_REDLINE, [255, 80, 90, 255])
        .with_colour(drive::TRACK, [255, 255, 255, 26])
        .with_colour(drive::GEAR_BOX, [255, 255, 255, 22])
        .with_colour(drive::GEAR_SHIFT, [255, 80, 90, 255])
        .with_colour(drive::GEAR_LETTER, [120, 220, 255, 255])
        .with_colour(drive::PIP_OFF, [255, 255, 255, 30])
        .with_colour(drive::PIP_ON, [120, 220, 255, 255])
        .with_colour(drive::NOS, [90, 200, 255, 255])
        .with_colour(drive::BOOST, [255, 170, 80, 255])
        .with_colour(drive::SPEEDBREAKER, [150, 255, 170, 255])
        .with_font(drive::FONT, RASTER_UPRIGHT_FONT_KEY)
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
