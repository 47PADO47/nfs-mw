//! Text drawn in a TrueType font: the one the egui overlay (the FPS counter) draws in. The game's own text is bitmap FEng
//! fonts, which blur when scaled up, so the Remastered theme names this font key on its text instead. The presenter
//! rasterises the string once and draws it as a texture (`ui/present/blackbox.rs`).

use std::sync::OnceLock;

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};

use super::assets::Image;

/// The skin font key the Remastered theme gives its text. Not a FEng font hash: the presenter checks for it first.
pub const RASTER_FONT_KEY: u32 = 0x5241_5354;
/// The Remastered theme's numbers: the same font, drawn heavier than its text.
pub const RASTER_NUMBER_FONT_KEY: u32 = 0x5241_5356;
/// The same font, drawn upright. The Custom theme gives its text this key, so it is not leaned like the Remastered text.
pub const RASTER_UPRIGHT_FONT_KEY: u32 = 0x5241_5355;
/// The monospace font the egui overlay (the FPS counter) draws in, upright. The Remastered map legend uses it so its
/// text matches that overlay instead of the game's leaned font.
pub const RASTER_MONO_FONT_KEY: u32 = 0x5241_5357;
/// The Remastered menus' text: upright, a little spaced. The entries of the vertical menus and their footer hints.
pub const RASTER_MENU_FONT_KEY: u32 = 0x5241_5358;
/// The same face spaced wide. The menus' title and tagline.
pub const RASTER_MENU_WIDE_FONT_KEY: u32 = 0x5241_5359;
/// The menus' title: the wide face, bold.
pub const RASTER_MENU_TITLE_FONT_KEY: u32 = 0x5241_535A;
/// A canvas unit is 2 pixels across and 1.5 down: a string's width in units is its height in units times the aspect times
/// this (`ui/present/blackbox`).
pub const UNITS_ASPECT: f32 = 1.5 / 2.0;
/// Canvas units of text height per unit of text scale.
pub const UNITS_PER_SCALE: f32 = 10.0;

/// The pixel height a string is rasterised at. It is downscaled to the size it is drawn at, so it stays sharp.
const RASTER_HEIGHT_PX: f32 = 128.0;
/// How a string is drawn: its width is stretched by `widen`, and its coverage is raised to `weight`. A weight below 1
/// makes the strokes heavier; 1 keeps them as the font draws them. `tracking` adds space after each glyph, as a fraction
/// of the font's height (before the stretch). `bold` thickens every stroke by that many pixels of the glyph image, which is
/// `RASTER_HEIGHT_PX` tall.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub widen: f32,
    pub weight: f32,
    pub tracking: f32,
    pub bold: u32,
}

/// The font as it is drawn: no stretch, no added weight. The Custom theme's text.
pub const PLAIN: Look = Look { widen: 1.0, weight: 1.0, tracking: 0.0, bold: 0 };
/// The Remastered theme's text: wider and heavier than the font's own proportions.
pub const REMASTERED: Look = Look { widen: 1.6, weight: 0.6, tracking: 0.0, bold: 0 };
/// The Remastered theme's numbers: as wide as its text, and heavier still.
pub const NUMBERS: Look = Look { widen: 1.6, weight: 0.4, tracking: 0.0, bold: 0 };
/// The Remastered menus' text: heavier than the font's own strokes, a little spaced.
pub const MENU: Look = Look { widen: 1.23, weight: 0.28, tracking: 0.05, bold: 0 };
/// The Remastered menus' title and tagline: the menu face spaced wide.
pub const MENU_WIDE: Look = Look { widen: 1.14, weight: 0.4, tracking: 0.3, bold: 0 };

/// The menus' title: spaced, and bold.
pub const MENU_TITLE: Look = Look { widen: 1.14, weight: 0.4, tracking: 0.2, bold: 4 };

/// How a font key is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub look: Look,
    /// The egui monospace font instead of the proportional one.
    pub mono: bool,
    /// Drawn upright; the Remastered cluster's text leans.
    pub upright: bool,
}

/// The face a skin font key names, or `None` for a key that is not rasterised text (a FEng font hash).
pub fn face(key: u32) -> Option<Face> {
    let (look, mono, upright) = match key {
        RASTER_FONT_KEY => (REMASTERED, false, false),
        RASTER_NUMBER_FONT_KEY => (NUMBERS, false, false),
        RASTER_UPRIGHT_FONT_KEY => (PLAIN, false, true),
        RASTER_MONO_FONT_KEY => (PLAIN, true, true),
        RASTER_MENU_FONT_KEY => (MENU, false, true),
        RASTER_MENU_WIDE_FONT_KEY => (MENU_WIDE, false, true),
        RASTER_MENU_TITLE_FONT_KEY => (MENU_TITLE, false, true),
        _ => return None,
    };
    Some(Face { look, mono, upright })
}

/// The width of a string drawn in the face `key` at text scale `scale`, in FEng units, as the presenter draws it. `None`
/// for a key that is not rasterised text, or when the font does not load.
pub fn width_units(key: u32, text: &str, scale: f32) -> Option<f32> {
    let face = face(key)?;
    let font = if face.mono { mono_font()? } else { font()? };
    let scaled = font.as_scaled(PxScale::from(RASTER_HEIGHT_PX));
    let height = (scaled.ascent() - scaled.descent()).ceil().max(1.0);
    let width = advance(font, text, face.look).ceil().max(1.0) * face.look.widen;
    Some(scale * UNITS_PER_SCALE * width.ceil() / height * UNITS_ASPECT)
}

/// The pen position after `text`: the sum of the glyph advances, each but the last followed by the look's tracking.
fn advance(font: &FontArc, text: &str, look: Look) -> f32 {
    let scaled = font.as_scaled(PxScale::from(RASTER_HEIGHT_PX));
    let last = text.chars().count().saturating_sub(1);
    text.chars().enumerate().map(|(k, ch)| scaled.h_advance(font.glyph_id(ch)) + gap(look, k, last)).sum()
}

/// The space after the glyph `k` of a string whose last glyph is `last`: the tracking, and none after the last one, so a
/// right-aligned string ends on its edge.
fn gap(look: Look, k: usize, last: usize) -> f32 {
    if k == last {
        return 0.0;
    }
    look.tracking * RASTER_HEIGHT_PX
}

/// The font, once: the egui default proportional font, bundled with egui.
static FONT: OnceLock<Option<FontArc>> = OnceLock::new();
/// The egui monospace font (Hack), once: the one the FPS overlay draws in.
static MONO_FONT: OnceLock<Option<FontArc>> = OnceLock::new();

fn font() -> Option<&'static FontArc> {
    FONT.get_or_init(|| FontArc::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT).ok()).as_ref()
}

fn mono_font() -> Option<&'static FontArc> {
    MONO_FONT.get_or_init(|| FontArc::try_from_slice(epaint_default_fonts::HACK_REGULAR).ok()).as_ref()
}

/// The string as a white image, in the proportional font. See [`render_with`].
pub fn render(text: &str, look: Look) -> Option<Image> {
    render_with(text, look, false)
}

/// The string as a white image, in the monospace font the FPS overlay uses.
pub fn render_mono(text: &str, look: Look) -> Option<Image> {
    render_with(text, look, true)
}

/// The string as a white image with its coverage in the alpha channel, `RASTER_HEIGHT_PX` tall, drawn as `look` says.
/// `None` when the font does not load or the string is empty.
fn render_with(text: &str, look: Look, mono: bool) -> Option<Image> {
    let font = if mono { mono_font()? } else { font()? };
    let scaled = font.as_scaled(PxScale::from(RASTER_HEIGHT_PX));
    let ascent = scaled.ascent();
    let height = (scaled.ascent() - scaled.descent()).ceil().max(1.0) as u32;
    let mut glyphs = Vec::new();
    let mut pen = 0.0_f32;
    let last = text.chars().count().saturating_sub(1);
    for (k, ch) in text.chars().enumerate() {
        let id = font.glyph_id(ch);
        glyphs.push(id.with_scale_and_position(scaled.scale(), ab_glyph::point(pen, ascent)));
        pen += scaled.h_advance(id) + gap(look, k, last);
    }
    let width = pen.ceil().max(1.0) as u32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for glyph in glyphs {
        let Some(outlined) = font.outline_glyph(glyph) else { continue };
        let bounds = outlined.px_bounds();
        outlined.draw(|x, y, coverage| {
            let px = bounds.min.x as i64 + i64::from(x);
            let py = bounds.min.y as i64 + i64::from(y);
            if px < 0 || py < 0 || px >= i64::from(width) || py >= i64::from(height) {
                return;
            }
            let at = (py as u32 * width + px as u32) as usize * 4;
            let alpha = (coverage.powf(look.weight) * 255.0).round() as u8;
            let kept = alpha.max(rgba[at + 3]);
            rgba[at..at + 4].copy_from_slice(&[255, 255, 255, kept]);
        });
    }
    embolden(&mut rgba, width as usize, height as usize, look.bold as usize);
    Some(stretch(Image { width, height, rgba, blend: 1 }, look.widen))
}

/// Thickens every stroke by `radius` pixels in all directions: each pixel takes the largest coverage within the radius.
/// Done in two passes, along the rows and then along the columns.
fn embolden(rgba: &mut [u8], width: usize, height: usize, radius: usize) {
    if radius == 0 {
        return;
    }
    spread(rgba, height, width, (width, 1), radius);
    spread(rgba, width, height, (1, width), radius);
}

/// One pass of [`embolden`]: `lines` lines of `len` pixels, a line `stride.0` apart and a pixel `stride.1` apart.
fn spread(rgba: &mut [u8], lines: usize, len: usize, stride: (usize, usize), radius: usize) {
    let coverage: Vec<u8> = rgba.iter().skip(3).step_by(4).copied().collect();
    for line in 0..lines {
        for at in 0..len {
            let (lo, hi) = (at.saturating_sub(radius), (at + radius).min(len - 1));
            let best = (lo..=hi).map(|k| coverage[line * stride.0 + k * stride.1]).max().unwrap_or(0);
            let index = line * stride.0 + at * stride.1;
            rgba[index * 4..index * 4 + 4].copy_from_slice(&[255, 255, 255, best]);
        }
    }
}

/// The image stretched horizontally to `factor` times its width, with the alpha interpolated between the source columns.
/// A factor of 1 returns the image as it is.
fn stretch(image: Image, factor: f32) -> Image {
    if factor == 1.0 {
        return image;
    }
    let (w, h) = (image.width as usize, image.height as usize);
    let new_w = (image.width as f32 * factor).ceil().max(1.0) as usize;
    let mut rgba = vec![0u8; new_w * h * 4];
    for y in 0..h {
        for x in 0..new_w {
            let src = ((x as f32 + 0.5) / factor - 0.5).clamp(0.0, (w - 1) as f32);
            let (x0, t) = (src.floor() as usize, src - src.floor());
            let x1 = (x0 + 1).min(w - 1);
            let alpha = |col: usize| f32::from(image.rgba[(y * w + col) * 4 + 3]);
            let kept = alpha(x0) * (1.0 - t) + alpha(x1) * t;
            let at = (y * new_w + x) * 4;
            rgba[at..at + 4].copy_from_slice(&[255, 255, 255, kept.round() as u8]);
        }
    }
    Image { width: new_w as u32, height: image.height, rgba, blend: image.blend }
}

#[cfg(test)]
mod tests {
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
            let from_image =
                1.0 * super::UNITS_PER_SCALE * image.width as f32 / image.height as f32 * super::UNITS_ASPECT;
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
    fn a_fengs_font_hash_is_not_rasterised_text() {
        assert!(super::face(0x5455_70c6).is_none());
        assert!(super::width_units(0x5455_70c6, "x", 1.0).is_none());
    }

    #[test]
    fn widening_stretches_the_string_and_keeps_its_height() {
        let plain = super::render("186", super::PLAIN).expect("the bundled font loads");
        let wide = super::render("186", super::REMASTERED).expect("the bundled font loads");
        assert_eq!(wide.height, plain.height);
        assert_eq!(wide.width, (plain.width as f32 * super::REMASTERED.widen).ceil() as u32);
    }
}
