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

/// The pixel height a string is rasterised at. It is downscaled to the size it is drawn at, so it stays sharp.
const RASTER_HEIGHT_PX: f32 = 128.0;
/// How a string is drawn: its width is stretched by `widen`, and its coverage is raised to `weight`. A weight below 1
/// makes the strokes heavier; 1 keeps them as the font draws them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub widen: f32,
    pub weight: f32,
}

/// The font as it is drawn: no stretch, no added weight. The Custom theme's text.
pub const PLAIN: Look = Look { widen: 1.0, weight: 1.0 };
/// The Remastered theme's text: wider and heavier than the font's own proportions.
pub const REMASTERED: Look = Look { widen: 1.6, weight: 0.6 };
/// The Remastered theme's numbers: as wide as its text, and heavier still.
pub const NUMBERS: Look = Look { widen: 1.6, weight: 0.4 };

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
    for ch in text.chars() {
        let id = font.glyph_id(ch);
        glyphs.push(id.with_scale_and_position(scaled.scale(), ab_glyph::point(pen, ascent)));
        pen += scaled.h_advance(id);
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
    Some(stretch(Image { width, height, rgba, blend: 1 }, look.widen))
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
    #[test]
    fn a_string_rasterises_with_coverage() {
        let image = super::render("186", super::PLAIN).expect("the bundled font loads");
        assert!(image.width > 0 && image.height > 0);
        assert!(image.rgba.as_chunks::<4>().0.iter().any(|px| px[3] > 200), "some glyph pixel is solid");
    }

    #[test]
    fn widening_stretches_the_string_and_keeps_its_height() {
        let plain = super::render("186", super::PLAIN).expect("the bundled font loads");
        let wide = super::render("186", super::REMASTERED).expect("the bundled font loads");
        assert_eq!(wide.height, plain.height);
        assert_eq!(wide.width, (plain.width as f32 * super::REMASTERED.widen).ceil() as u32);
    }
}
