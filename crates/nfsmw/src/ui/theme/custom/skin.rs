//! The Custom skin: the colours and font the radio card reads, by key. `ui/theme/mod.rs` hands this skin to every
//! component, so the keys are namespaced by element (`radio.*`); the Original components never read them.

use blackbox_feng::fe_hash_upper;
use blackbox_ui::Skin;

/// The skin keys the radio card reads.
pub(super) mod keys {
    pub const BACKGROUND: &str = "radio.background";
    pub const TITLE: &str = "radio.title";
    pub const CREDIT: &str = "radio.credit";
    pub const STATUS: &str = "radio.status";
    pub const FONT: &str = "radio.font";
}

/// The Custom skin. The card's colours are the egui card's: a black panel at 150/255 alpha, white title, grey artist and
/// album, and a dimmer grey time line. The font is the HUD body font (`FONT_MW_BODY`, the prompts' font too).
pub fn card_skin() -> Skin {
    Skin::new()
        .with_colour(keys::BACKGROUND, [0, 0, 0, 150])
        .with_colour(keys::TITLE, [255, 255, 255, 255])
        .with_colour(keys::CREDIT, [200, 200, 200, 255])
        .with_colour(keys::STATUS, [150, 150, 150, 255])
        .with_font(keys::FONT, fe_hash_upper("FONT_MW_BODY"))
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

    #[test]
    fn the_font_is_the_prompt_body_font() {
        // `frontend/prompts.rs` names FONT_MW_BODY by its hash; the card must read the same font.
        assert_eq!(card_skin().font(keys::FONT), Some(0x5455_70c6));
    }

    #[test]
    fn fading_scales_only_the_alpha_of_the_card_keys() {
        let faded = faded(&card_skin(), 0.5);
        assert_eq!(faded.colour(keys::BACKGROUND), Some([0, 0, 0, 75]));
        assert_eq!(faded.colour(keys::TITLE), Some([255, 255, 255, 128]));
        assert_eq!(faded.font(keys::FONT), card_skin().font(keys::FONT));
        assert_eq!(faded.colour("text"), None);
    }
}
