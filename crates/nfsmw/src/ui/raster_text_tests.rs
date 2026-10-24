//! The rasterised text: its widths, tracking, weight and fallback, in the bundled egui fonts.

use super::{RASTER_FONT_KEY, RASTER_MENU_FONT_KEY, RASTER_MENU_WIDE_FONT_KEY};

#[test]
fn a_string_rasterises_with_coverage() {
    let image = super::render("186", super::PLAIN).expect("the bundled font loads");
    assert!(image.width > 0 && image.height > 0);
    assert!(image.rgba.as_chunks::<4>().0.iter().any(|px| px[3] > 200), "some glyph pixel is solid");
}

#[test]
fn tracking_spaces_the_glyphs_but_not_the_end() {
    let plain = super::render("MENU", super::PLAIN).expect("the bundled font loads");
    let spaced = super::render("MENU", super::Look { tracking: 0.25, ..super::PLAIN }).expect("the font loads");
    let extra = (0.25 * super::RASTER_HEIGHT_PX * 3.0) as u32;
    assert!(spaced.width.abs_diff(plain.width + extra) <= 1, "three gaps in four glyphs");
}

#[test]
fn the_measured_width_matches_the_rendered_image() {
    for key in [RASTER_MENU_FONT_KEY, RASTER_MENU_WIDE_FONT_KEY, RASTER_FONT_KEY] {
        let face = super::face(key).expect("a raster key");
        let image = super::render("QUICK RACE", face.look).expect("the bundled font loads");
        let from_image = 1.0 * super::UNITS_PER_SCALE * image.width as f32 / image.height as f32 * super::UNITS_ASPECT;
        let measured = super::width_units(key, "QUICK RACE", 1.0).expect("a measurable key");
        assert!((measured - from_image).abs() < 1e-3, "{key:#x}: {measured} against {from_image}");
    }
}

#[test]
fn bold_thickens_the_strokes_and_keeps_the_size() {
    let plain = super::render("I", super::PLAIN).expect("the bundled font loads");
    let bold = super::render("I", super::Look { bold: 3, ..super::PLAIN }).expect("the bundled font loads");
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
    assert!(super::width_units(0x5455_70c6, "x", 1.0).is_none());
}

#[test]
fn widening_stretches_the_string_and_keeps_its_height() {
    let plain = super::render("186", super::PLAIN).expect("the bundled font loads");
    let look = super::Look { tracking: 0.0, ..super::REMASTERED };
    let wide = super::render("186", look).expect("the bundled font loads");
    assert_eq!(wide.height, plain.height);
    assert_eq!(wide.width, (plain.width as f32 * look.widen).ceil() as u32);
}
