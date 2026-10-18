//! Text drawn in a TrueType font: the one the egui overlay (the FPS counter) draws in. The game's own text is bitmap FEng
//! fonts, which blur when scaled up, so the Remastered theme names this font key on its text instead. The presenter
//! rasterises the string once and draws it as a texture (`ui/present/blackbox.rs`).

use std::sync::OnceLock;

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};

use super::assets::Image;

/// The skin font key the Remastered theme gives its text. Not a FEng font hash: the presenter checks for it first.
pub const RASTER_FONT_KEY: u32 = 0x5241_5354;
/// The same font, drawn upright. The Custom theme gives its text this key, so it is not leaned like the Remastered text.
pub const RASTER_UPRIGHT_FONT_KEY: u32 = 0x5241_5355;

/// The pixel height a string is rasterised at. It is downscaled to the size it is drawn at, so it stays sharp.
const RASTER_HEIGHT_PX: f32 = 128.0;

/// The font, once: the egui default proportional font, bundled with egui.
static FONT: OnceLock<Option<FontArc>> = OnceLock::new();

fn font() -> Option<&'static FontArc> {
    FONT.get_or_init(|| FontArc::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT).ok()).as_ref()
}

/// The string as a white image with its coverage in the alpha channel, `RASTER_HEIGHT_PX` tall. `None` when the font
/// does not load or the string is empty.
pub fn render(text: &str) -> Option<Image> {
    let font = font()?;
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
            let alpha = (coverage * 255.0).round() as u8;
            let kept = alpha.max(rgba[at + 3]);
            rgba[at..at + 4].copy_from_slice(&[255, 255, 255, kept]);
        });
    }
    Some(Image { width, height, rgba, blend: 1 })
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_string_rasterises_with_coverage() {
        let image = super::render("186").expect("the bundled font loads");
        assert!(image.width > 0 && image.height > 0);
        assert!(image.rgba.chunks_exact(4).any(|px| px[3] > 200), "some glyph pixel is solid");
    }
}
