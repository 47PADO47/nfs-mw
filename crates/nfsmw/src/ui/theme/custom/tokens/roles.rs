//! The theme's colour roles as the player's `[custom_hud]` look needs them (`docs/specs/custom-hud/map.md` "Colours").
//!
//! The map and the other token-driven parts draw in the active theme's roles (`blackbox_ui::Roles`, skin keys
//! `theme.line`, `theme.guide`, ...): a theme brings its own look, a mod theme's derived roles included. The player's
//! accessibility settings still win over the theme, in the same order the `custom.*` colours apply them:
//!
//! 1. **Colour-vision palette.** The meaning roles (`guide`, `success`, `caution`, `danger`) take the palette's hues
//!    (`Colours::for_look`), so the colours the palette keeps apart stay apart whatever the theme chose. The default
//!    palette leaves the theme's own hues.
//! 2. **High contrast.** The linework turns white on near-black, the scrim and the overlay darken, and every meaning
//!    hue brightens (the spec's tuned hue for the built-in ones, else moved towards white).
//!
//! `info` carries no meaning a colour-vision deficiency could lose (it always comes with a shape or a rhythm), so it keeps
//! the theme's hue, brightened by high contrast.

use blackbox_ui::{Skin, Tokens, token_keys};

use super::CustomLook;
use super::colours::{Colours, brighten, high_contrast_hue};
use crate::settings::hud::HudPalette;

/// The scrim and the overlay under high contrast: darker, so the map reads against any scene.
pub const HIGH_CONTRAST_SCRIM: [u8; 4] = [0, 0, 0, 205];
pub const HIGH_CONTRAST_OVERLAY: [u8; 4] = [0, 0, 0, 245];

/// The `theme.*` role entries for `tokens` under `look`. Merged over the tokens' own skin by `UiThemes::refresh_skin`.
/// The linework, scrim and overlay are only here with high contrast: without it they are the tokens' own entries.
pub fn roles_skin(tokens: &Tokens, look: CustomLook) -> Skin {
    let roles = tokens.resolved_roles();
    let palette = Colours::for_look(look);
    let hue = |theme: [u8; 4], chosen: [u8; 4]| match (look.palette, look.high_contrast) {
        (HudPalette::Default, false) => theme,
        (HudPalette::Default, true) => high_contrast_hue(theme),
        _ => chosen,
    };
    let info = match look.high_contrast {
        true => brighten(roles.info),
        false => roles.info,
    };
    let skin = Skin::new()
        .with_colour(token_keys::GUIDE, hue(roles.guide, palette.nav))
        .with_colour(token_keys::SUCCESS, hue(roles.success, palette.success))
        .with_colour(token_keys::CAUTION, hue(roles.caution, palette.caution))
        .with_colour(token_keys::DANGER, hue(roles.danger, palette.danger))
        .with_colour(token_keys::INFO, info);
    if !look.high_contrast {
        return skin;
    }
    skin.with_colour(token_keys::LINE, palette.text)
        .with_colour(token_keys::LINE_MINOR, palette.text_secondary)
        .with_colour(token_keys::LINE_FAINT, palette.rule)
        .with_colour(token_keys::SCRIM, HIGH_CONTRAST_SCRIM)
        .with_colour(token_keys::OVERLAY, HIGH_CONTRAST_OVERLAY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::custom::palette;

    fn look(palette: HudPalette, high_contrast: bool) -> CustomLook {
        CustomLook { palette, high_contrast }
    }

    fn of(tokens: &Tokens, look: CustomLook, key: &str) -> [u8; 4] {
        roles_skin(tokens, look).colour(key).expect("the role is in the skin")
    }

    #[test]
    fn the_default_look_leaves_the_themes_own_roles() {
        let tokens = palette::bevy();
        let skin = roles_skin(&tokens, CustomLook::default());
        let resolved = tokens.resolved_roles();
        assert_eq!(skin.colour(token_keys::GUIDE), Some(resolved.guide));
        assert_eq!(skin.colour(token_keys::DANGER), Some(resolved.danger));
        assert_eq!(skin.colour(token_keys::LINE), None, "the neutrals are the tokens' own skin entries");
    }

    #[test]
    fn the_builtin_palettes_roles_are_the_custom_briefs_colours() {
        let brief = Colours::BRIEF;
        for tokens in [palette::bevy(), palette::remastered()] {
            let roles = tokens.resolved_roles();
            assert_eq!(
                (roles.line, roles.line_minor, roles.line_faint),
                (brief.text, brief.text_secondary, brief.rule)
            );
            assert_eq!((roles.success, roles.caution, roles.danger), (brief.success, brief.caution, brief.danger));
        }
        assert_eq!(palette::bevy().resolved_roles().guide, brief.nav, "the default look's route is the brief's");
    }

    #[test]
    fn a_colour_vision_palette_replaces_the_meaning_hues_whatever_the_theme_chose() {
        let mut tokens = palette::bevy();
        tokens.roles.danger = Some([1, 2, 3, 255]);
        tokens.roles.guide = Some([4, 5, 6, 255]);
        let deut = look(HudPalette::Deuteranopia, false);
        let expected = Colours::for_look(deut);
        assert_eq!(of(&tokens, deut, token_keys::DANGER), expected.danger);
        assert_eq!(of(&tokens, deut, token_keys::GUIDE), expected.nav);
        assert_eq!(of(&tokens, deut, token_keys::SUCCESS), expected.success);
        assert_eq!(of(&tokens, deut, token_keys::CAUTION), expected.caution);
    }

    #[test]
    fn high_contrast_uses_the_specs_tuned_hues_for_the_builtin_palette_and_brightens_a_themes_own() {
        let tokens = palette::bevy();
        let high = look(HudPalette::Default, true);
        let spec = Colours::for_look(high);
        assert_eq!(of(&tokens, high, token_keys::GUIDE), spec.nav);
        assert_eq!(of(&tokens, high, token_keys::DANGER), spec.danger);
        let mut own = palette::bevy();
        own.roles.caution = Some([100, 100, 100, 255]);
        assert_eq!(of(&own, high, token_keys::CAUTION), brighten([100, 100, 100, 255]));
    }

    #[test]
    fn high_contrast_turns_the_linework_white_and_darkens_the_backdrop() {
        let tokens = palette::bevy();
        let normal = tokens.resolved_roles();
        let high = look(HudPalette::Default, true);
        assert_eq!(of(&tokens, high, token_keys::LINE), [255, 255, 255, 255]);
        assert!(of(&tokens, high, token_keys::SCRIM)[3] > normal.scrim[3], "a stronger scrim");
        assert!(of(&tokens, high, token_keys::OVERLAY)[3] > normal.overlay[3], "a stronger plate");
    }

    #[test]
    fn a_colour_vision_palette_under_high_contrast_is_the_brightened_palette() {
        let tokens = palette::remastered();
        let both = look(HudPalette::Tritanopia, true);
        assert_eq!(of(&tokens, both, token_keys::SUCCESS), Colours::for_look(both).success);
    }

    #[test]
    fn a_mod_theme_with_no_roles_still_gets_a_complete_set() {
        let plain = Tokens::default();
        let keys = [token_keys::GUIDE, token_keys::SUCCESS, token_keys::CAUTION, token_keys::DANGER, token_keys::INFO];
        for high in [false, true] {
            let skin = roles_skin(&plain, look(HudPalette::Default, high));
            for key in keys {
                assert!(skin.colour(key).is_some(), "{key} with high contrast {high}");
            }
        }
    }
}
