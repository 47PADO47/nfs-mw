//! The rasterised text: its widths, tracking, weight, tabular digits and fallback, in the bundled egui fonts. The Custom
//! keys' system fonts differ between machines, so only what holds in any font is checked against them.

use super::{
    CUSTOM_LABEL_FONT_KEY, CUSTOM_LABEL_WIDE_FONT_KEY, CUSTOM_NUMERIC_FONT_KEY, Face, Image, Look, RASTER_FONT_KEY,
    RASTER_MENU_FONT_KEY, RASTER_MENU_WIDE_FONT_KEY, Role, Source,
};

/// The string in the bundled proportional font, drawn as `look` says.
fn render(text: &str, look: Look) -> Option<Image> {
    Some(super::render_with(text, look, super::font()?, false))
}

#[test]
fn a_string_rasterises_with_coverage() {
    let image = render("186", super::PLAIN).expect("the bundled font loads");
    assert!(image.width > 0 && image.height > 0);
    assert!(image.rgba.as_chunks::<4>().0.iter().any(|px| px[3] > 200), "some glyph pixel is solid");
}

#[test]
fn tracking_spaces_the_glyphs_but_not_the_end() {
    let plain = render("MENU", super::PLAIN).expect("the bundled font loads");
    let spaced = render("MENU", super::Look { tracking: 0.25, ..super::PLAIN }).expect("the font loads");
    let extra = (0.25 * super::RASTER_HEIGHT_PX * 3.0) as u32;
    assert!(spaced.width.abs_diff(plain.width + extra) <= 1, "three gaps in four glyphs");
}

#[test]
fn the_measured_width_matches_the_rendered_image() {
    for key in [RASTER_MENU_FONT_KEY, RASTER_MENU_WIDE_FONT_KEY, RASTER_FONT_KEY] {
        let face = super::face(key).expect("a raster key");
        let image = super::render("QUICK RACE", face).expect("the bundled font loads");
        let from_image = 1.0 * super::UNITS_PER_SCALE * image.width as f32 / image.height as f32 * super::UNITS_ASPECT;
        let measured = super::width_units(key, "QUICK RACE", 1.0).expect("a measurable key");
        assert!((measured - from_image).abs() < 1e-3, "{key:#x}: {measured} against {from_image}");
    }
}

#[test]
fn the_custom_keys_measure_what_they_draw() {
    for key in [CUSTOM_NUMERIC_FONT_KEY, CUSTOM_LABEL_FONT_KEY, CUSTOM_LABEL_WIDE_FONT_KEY] {
        let face = super::face(key).expect("a raster key");
        let image = super::render("LAP 1:07.41", face).expect("a system font or the bundled one loads");
        let from_image = super::UNITS_PER_SCALE * image.width as f32 / image.height as f32 * super::UNITS_ASPECT;
        let measured = super::width_units(key, "LAP 1:07.41", 1.0).expect("a measurable key");
        assert!((measured - from_image).abs() < 1e-3, "{key:#x}: {measured} against {from_image}");
    }
}

#[test]
fn the_custom_keys_name_the_system_roles_upright() {
    let roles = [
        (CUSTOM_NUMERIC_FONT_KEY, Role::Numeric),
        (CUSTOM_LABEL_FONT_KEY, Role::Label),
        (CUSTOM_LABEL_WIDE_FONT_KEY, Role::LabelWide),
    ];
    for (key, role) in roles {
        let face = super::face(key).expect("a raster key");
        assert_eq!(face.source, Source::System(role), "{key:#x}");
        assert!(face.upright, "{key:#x}: the Custom text does not lean");
        let looks = [super::role_look(role, true), super::role_look(role, false)];
        assert!(looks.contains(&face.look), "{key:#x}: the system look or the narrowed fallback");
        assert_eq!(face.tabular(), role == Role::Numeric, "{key:#x}: only the numbers are tabular");
    }
}

#[test]
fn the_remastered_keys_keep_their_faces() {
    let face = super::face(RASTER_FONT_KEY).expect("a raster key");
    assert_eq!(face, Face { look: super::REMASTERED, source: Source::Egui, upright: false });
    let mono = super::face(super::RASTER_MONO_FONT_KEY).expect("a raster key");
    assert_eq!(mono, Face { look: super::PLAIN, source: Source::EguiMono, upright: true });
    assert!(!face.tabular() && !mono.tabular());
}

#[test]
fn the_fallback_narrows_the_bundled_font() {
    for role in Role::ALL {
        let fallback = super::role_look(role, false);
        assert!(fallback.widen < 1.0, "{role:?}: the bundled font is narrowed to imitate a condensed face");
        let system = super::role_look(role, true);
        assert!((system.widen - 1.0).abs() < 1e-6, "{role:?}: a system font keeps its own proportions");
    }
}

#[test]
fn tabular_digits_share_the_advance_of_the_zero() {
    let font = super::font().expect("the bundled font loads");
    let zero = super::cell(font, '0', true);
    for digit in '0'..='9' {
        assert!((super::cell(font, digit, true) - zero).abs() < 1e-4, "{digit}");
    }
    let own = |ch| super::cell(font, ch, false);
    assert!((super::cell(font, 'i', true) - own('i')).abs() < 1e-4, "letters keep their own advance");
    let ones = super::advance(font, "1111", super::PLAIN, true);
    let eights = super::advance(font, "8888", super::PLAIN, true);
    assert!((ones - eights).abs() < 1e-3, "four digits are as wide whatever they are");
}

#[test]
fn tabular_digits_draw_as_wide_as_they_measure() {
    let font = super::font().expect("the bundled font loads");
    let ones = super::render_with("1111", super::PLAIN, font, true);
    let zeros = super::render_with("0000", super::PLAIN, font, true);
    assert_eq!(ones.width, zeros.width);
    assert_eq!(ones.width, super::advance(font, "1111", super::PLAIN, true).ceil() as u32);
    assert!(ones.rgba.as_chunks::<4>().0.iter().any(|px| px[3] > 200), "the centred digits still draw");
}

#[test]
fn bold_thickens_the_strokes_and_keeps_the_size() {
    let plain = render("I", super::PLAIN).expect("the bundled font loads");
    let bold = render("I", super::Look { bold: 3, ..super::PLAIN }).expect("the bundled font loads");
    assert_eq!((bold.width, bold.height), (plain.width, plain.height));
    let covered = |image: &super::Image| image.rgba.as_chunks::<4>().0.iter().filter(|px| px[3] > 128).count();
    assert!(covered(&bold) > covered(&plain) * 3 / 2, "a bold stroke covers clearly more");
}

#[test]
fn fallback_text_draws_letters_the_game_fonts_lack() {
    let image = super::render_fallback("Привет").expect("the bundled font has Cyrillic");
    assert!(image.width > image.height, "six letters side by side");
    assert!(image.rgba.as_chunks::<4>().0.iter().any(|px| px[3] > 200));
}

#[test]
fn a_fengs_font_hash_is_not_rasterised_text() {
    assert!(super::face(0x5455_70c6).is_none());
    assert!(super::face(0x5241_535F).is_none(), "the key after the Custom ones is free");
    assert!(super::width_units(0x5455_70c6, "x", 1.0).is_none());
}

#[test]
fn widening_stretches_the_string_and_keeps_its_height() {
    let plain = render("186", super::PLAIN).expect("the bundled font loads");
    let look = super::Look { tracking: 0.0, ..super::REMASTERED };
    let wide = render("186", look).expect("the bundled font loads");
    assert_eq!(wide.height, plain.height);
    assert_eq!(wide.width, (plain.width as f32 * look.widen).ceil() as u32);
}
