//! Tests for the Custom tokens: the brief's values, high contrast, the colour-vision palettes, the skin entries,
//! opacity and the motion table.

use blackbox_ui::Skin;

use super::colours::{BRIGHTEN, brighten};
use super::{Colours, CustomLook, Motion, SAFETY_OPACITY, fade, faded_skin, keys, ramp, safety_opacity, skin};
use crate::settings::hud::{HudPalette, HudSettings};
use crate::ui::raster_text::{CUSTOM_LABEL_FONT_KEY, CUSTOM_NUMERIC_FONT_KEY};

const PALETTES: [HudPalette; 3] = [HudPalette::Default, HudPalette::Deuteranopia, HudPalette::Tritanopia];
const CYAN: [u8; 4] = [120, 220, 255, 255];

fn look(palette: HudPalette, high_contrast: bool) -> CustomLook {
    CustomLook { palette, high_contrast }
}

/// The neutrals of a colour set: the values a palette must not change.
fn neutrals(c: &Colours) -> [[u8; 4]; 5] {
    [c.text, c.text_secondary, c.surface, c.surface_raised, c.rule]
}

/// The meaning hues of a colour set: the values a palette replaces.
fn hues(c: &Colours) -> [[u8; 4]; 5] {
    [c.nav, c.success, c.caution, c.danger, c.critical]
}

#[test]
fn the_default_look_is_the_briefs_palette() {
    let c = Colours::for_look(CustomLook::default());
    assert_eq!(c, Colours::BRIEF);
    assert_eq!(c.text, [0xE8, 0xED, 0xF2, 255], "#E8EDF2");
    assert_eq!(c.text_secondary, [0x9B, 0xA7, 0xB3, 255], "#9BA7B3");
    assert_eq!(c.danger, [0xE0, 0x52, 0x4D, 255], "#E0524D");
    assert_eq!(c.surface[3], 153, "the charcoal plate is translucent (0.6)");
    assert_eq!(c.outline, None, "no outline without high contrast");
}

#[test]
fn high_contrast_changes_the_values_and_adds_an_outline() {
    for palette in PALETTES {
        let normal = Colours::for_look(look(palette, false));
        let high = Colours::for_look(look(palette, true));
        assert_ne!(neutrals(&normal), neutrals(&high), "{palette:?}: the neutrals change");
        assert!(high.surface[3] > normal.surface[3], "{palette:?}: the plate is more opaque");
        assert_eq!(high.text, [255, 255, 255, 255]);
        assert!(high.outline.is_some(), "{palette:?}: plates are outlined");
        for (n, h) in hues(&normal).iter().zip(hues(&high)) {
            let sum = |c: &[u8; 4]| u32::from(c[0]) + u32::from(c[1]) + u32::from(c[2]);
            assert!(sum(&h) >= sum(n), "{palette:?}: a hue is darker under high contrast ({n:?} -> {h:?})");
        }
    }
    // The default palette's high-contrast hues are the spec's table.
    let high = Colours::for_look(look(HudPalette::Default, true));
    assert_eq!(high.danger, [0xFF, 0x6B, 0x66, 255]);
    assert_eq!(high.critical_text, [0, 0, 0, 255]);
}

#[test]
fn a_palette_swaps_only_the_meaning_hues() {
    let base = Colours::for_look(look(HudPalette::Default, false));
    for palette in [HudPalette::Deuteranopia, HudPalette::Tritanopia] {
        let swapped = Colours::for_look(look(palette, false));
        assert_eq!(neutrals(&swapped), neutrals(&base), "{palette:?} changed a neutral");
        assert_eq!(swapped.critical_text, base.critical_text);
        assert_ne!(hues(&swapped), hues(&base), "{palette:?} changed no hue");
    }
    let deuteranopia = Colours::for_look(look(HudPalette::Deuteranopia, false));
    assert_eq!(deuteranopia.success, [0x56, 0xB4, 0xE9, 255], "sky blue for escaped");
    assert_eq!(deuteranopia.danger, [0xD5, 0x5E, 0x00, 255], "vermilion for pursuit");
    let tritanopia = Colours::for_look(look(HudPalette::Tritanopia, false));
    assert_eq!(tritanopia.danger, base.danger, "tritanopia keeps the brief's danger");
}

#[test]
fn every_palette_keeps_its_signal_pairs_apart() {
    for palette in PALETTES {
        for high_contrast in [false, true] {
            let c = Colours::for_look(look(palette, high_contrast));
            assert_ne!(c.success, c.danger, "{palette:?}: escaped against pursuit");
            assert_ne!(c.caution, c.danger, "{palette:?}: shift point against red line");
            assert_ne!(c.nav, c.success, "{palette:?}: route against confirmation");
        }
    }
}

#[test]
fn high_contrast_brightens_a_colour_vision_palette_towards_white() {
    let plain = Colours::for_look(look(HudPalette::Tritanopia, false));
    let high = Colours::for_look(look(HudPalette::Tritanopia, true));
    assert_eq!(high.success, brighten(plain.success));
    assert_eq!(brighten([0, 0, 0, 7]), [64, 64, 64, 7], "{BRIGHTEN} of the way, alpha kept");
    assert_eq!(brighten([255, 255, 255, 255]), [255, 255, 255, 255]);
}

#[test]
fn the_skin_carries_every_key_the_accent_and_the_fonts() {
    let s = skin(CustomLook::default(), CYAN);
    let c = Colours::BRIEF;
    assert_eq!(s.colour(keys::TEXT), Some(c.text));
    assert_eq!(s.colour(keys::SURFACE), Some(c.surface));
    assert_eq!(s.colour(keys::NAV), Some(c.nav));
    assert_eq!(s.colour(keys::CRITICAL_TEXT), Some(c.critical_text));
    assert_eq!(s.colour(keys::ACCENT), Some(CYAN), "the accent is the theme's");
    assert_eq!(s.colour(keys::OUTLINE), None, "no outline without high contrast");
    assert_eq!(s.font(keys::FONT_NUMERIC), Some(CUSTOM_NUMERIC_FONT_KEY));
    assert_eq!(s.font(keys::FONT_LABEL), Some(CUSTOM_LABEL_FONT_KEY));
    let high = skin(look(HudPalette::Default, true), CYAN);
    assert_eq!(high.colour(keys::OUTLINE), Some([255, 255, 255, 153]));
    assert_eq!(high.colour(keys::ACCENT), Some(CYAN), "high contrast keeps the accent family");
}

#[test]
fn the_look_follows_the_settings() {
    let settings = HudSettings { palette: HudPalette::Deuteranopia, high_contrast: true, ..HudSettings::default() };
    assert_eq!(CustomLook::of(&settings), look(HudPalette::Deuteranopia, true));
    assert_eq!(CustomLook::of(&HudSettings::default()), CustomLook::default());
}

#[test]
fn fading_scales_only_the_alpha() {
    assert_eq!(fade([10, 20, 30, 200], 0.5), [10, 20, 30, 100]);
    assert_eq!(fade([10, 20, 30, 200], 2.0), [10, 20, 30, 200], "an opacity above 1 is 1");
    assert_eq!(fade([10, 20, 30, 200], -1.0), [10, 20, 30, 0]);
    assert_eq!(fade([10, 20, 30, 200], f32::NAN), [10, 20, 30, 0]);
    let base = Skin::new().with_colour("a", [1, 2, 3, 100]).with_font("f", 9).with_colour("other", [9; 4]);
    let faded = faded_skin(&base, &["a", "missing"], &["f"], 0.5);
    assert_eq!(faded.colour("a"), Some([1, 2, 3, 50]));
    assert_eq!(faded.font("f"), Some(9));
    assert_eq!(faded.colour("missing"), None);
    assert_eq!(faded.colour("other"), None, "only the keys asked for");
}

#[test]
fn a_safety_message_never_drops_below_its_floor() {
    assert_eq!(safety_opacity(0.2), SAFETY_OPACITY);
    assert_eq!(safety_opacity(0.95), 0.95);
}

#[test]
fn motion_reduction_removes_movement_and_keeps_the_timings_of_the_readings() {
    let full = Motion::of(false);
    let reduced = Motion::for_settings(&HudSettings { motion_reduction: true, ..HudSettings::default() });
    assert_eq!(full, Motion::FULL);
    assert_eq!(reduced, Motion::REDUCED);
    assert_eq!((full.fade_in, full.fade_out, full.slide, full.pulse), (0.150, 0.250, 0.200, 0.400));
    assert_eq!(reduced.slide_distance, 0.0, "a slide becomes a fade in place");
    assert!(!reduced.pulse_moves, "a pulse becomes a static highlight");
    assert_eq!(reduced.pulse, full.pulse, "held as long as the pulse");
    assert_eq!((reduced.mode_change, reduced.map_mode, reduced.map_zoom), (0.150, 0.150, 0.150));
    assert_eq!(reduced.speed_spring, full.speed_spring, "the speed scale keeps its spring");
    assert_eq!(reduced.fade_in, full.fade_in);
}

#[test]
fn a_ramp_runs_from_0_to_1_over_its_duration() {
    assert_eq!(ramp(0.0, 0.2), 0.0);
    assert!((ramp(0.1, 0.2) - 0.5).abs() < 1e-6);
    assert_eq!(ramp(1.0, 0.2), 1.0);
    assert_eq!(ramp(-1.0, 0.2), 0.0);
    assert_eq!(ramp(0.0, 0.0), 1.0, "no duration is already done");
}
