//! A name drawn into a picture. The car screens name a stock car with two pictures (its maker's logo and its model name, drawn
//! by the game's artists), and a car a mod added has neither, so the screens show a picture made of its name instead.
//!
//! The picture is white text with a soft dark shadow, leaned like the game's model-name pictures, centred in a picture of the
//! shape of the slot it goes in, so the screen needs no layout of its own: it sets the picture as the logo's texture.

use blackbox_feng::fe_hash_upper;
use blackbox_tpk::{AlphaUsage, PixelFormat, Texture};

use super::assets::Image;
use super::raster_text::{self, Face, Look, Source};

/// The strokes: heavy and a little wide, as the game's model names are.
const LOOK: Look = Look { widen: 1.12, weight: 0.3, tracking: 0.03, bold: 4 };
/// How far the top of a letter leans to the right, as a share of the letter's height.
const LEAN: f32 = 0.2;
/// The share of the picture's width and height the text may fill.
const FILL_WIDTH: f32 = 0.94;
const FILL_HEIGHT: f32 = 0.62;
/// The shadow's offset as a share of the picture's height, and how dark it is.
const SHADOW_OFFSET: f32 = 0.02;
const SHADOW_STRENGTH: f32 = 0.55;

/// The texture named `name`: `text` in white, shrunk to fit a picture of `width` by `height` pixels and centred in it. The
/// picture is clear when the text is empty or no font loads.
pub(crate) fn texture(name: &str, text: &str, (width, height): (u32, u32)) -> Texture {
    let mut data = vec![0u8; (width * height * 4) as usize];
    let drawn = Some(text.trim())
        .filter(|text| !text.is_empty())
        .and_then(|text| raster_text::render(text, Face { look: LOOK, source: Source::Egui, upright: false }));
    if let Some(glyphs) = drawn {
        paint(&mut data, &glyphs, width, height);
    }
    picture(name, (width, height), data)
}

/// A BGRA picture as a UI texture.
fn picture(name: &str, (width, height): (u32, u32), data: Vec<u8>) -> Texture {
    Texture {
        name: name.to_owned(),
        name_hash: fe_hash_upper(name),
        width,
        height,
        mip_levels: 1,
        format: PixelFormat::Argb8888,
        compression_type: 32,
        alpha_usage: AlphaUsage::Modulated,
        alpha_sorting: false,
        alpha_blend: 1,
        flags: 0,
        data,
        palette: Vec::new(),
    }
}

/// The texture named `name`: the logo `rgba` (`width` by `height`, straight alpha) shrunk to fit a picture of `slot` pixels and
/// centred in it, so a logo of any shape fills the slot the screen draws it in without stretching.
pub(crate) fn logo(name: &str, (width, height, rgba): (u32, u32, &[u8]), slot: (u32, u32)) -> Texture {
    let (sw, sh) = (slot.0 as f32, slot.1 as f32);
    let scale = (sw / width as f32).min(sh / height as f32);
    let (drawn_w, drawn_h) = (width as f32 * scale, height as f32 * scale);
    let (left, top) = ((sw - drawn_w) / 2.0, (sh - drawn_h) / 2.0);
    let mut data = vec![0u8; (slot.0 * slot.1 * 4) as usize];
    let texel = |x: i64, y: i64| -> [f32; 4] {
        if x < 0 || y < 0 || x >= i64::from(width) || y >= i64::from(height) {
            return [0.0; 4];
        }
        let at = ((y as u32 * width + x as u32) * 4) as usize;
        [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]].map(f32::from)
    };
    for y in 0..slot.1 {
        for x in 0..slot.0 {
            let u = (x as f32 + 0.5 - left) / scale - 0.5;
            let v = (y as f32 + 0.5 - top) / scale - 0.5;
            let (x0, y0) = (u.floor(), v.floor());
            let (fx, fy) = (u - x0, v - y0);
            let (x0, y0) = (x0 as i64, y0 as i64);
            let mut mixed = [0.0f32; 4];
            let corners =
                [(0, 0, (1.0 - fx) * (1.0 - fy)), (1, 0, fx * (1.0 - fy)), (0, 1, (1.0 - fx) * fy), (1, 1, fx * fy)];
            for (dx, dy, weight) in corners {
                let texel = texel(x0 + dx, y0 + dy);
                // Colours are weighted by their alpha, so a clear edge pixel does not tint its neighbours.
                let alpha = texel[3] / 255.0;
                for (sum, channel) in
                    mixed.iter_mut().zip([texel[0] * alpha, texel[1] * alpha, texel[2] * alpha, texel[3]])
                {
                    *sum += weight * channel;
                }
            }
            let alpha = mixed[3];
            if alpha <= 0.0 {
                continue;
            }
            let colour = |channel: f32| (channel / (alpha / 255.0)).round().clamp(0.0, 255.0) as u8;
            let at = ((y * slot.0 + x) * 4) as usize;
            data[at..at + 4].copy_from_slice(&[
                colour(mixed[2]),
                colour(mixed[1]),
                colour(mixed[0]),
                alpha.round() as u8,
            ]);
        }
    }
    picture(name, slot, data)
}

/// Where the glyph image lands in the picture: its top left corner and the scale from glyph pixels to picture pixels.
struct Placement {
    left: f32,
    top: f32,
    scale: f32,
    height: f32,
}

impl Placement {
    /// The glyph image `glyphs` shrunk to fit `width` by `height` with its lean, and centred.
    fn fit(glyphs: &Image, width: u32, height: u32) -> Self {
        let (gw, gh) = (glyphs.width as f32, glyphs.height as f32);
        let by_width = width as f32 * FILL_WIDTH / (gw + LEAN * gh);
        let by_height = height as f32 * FILL_HEIGHT / gh;
        let scale = by_width.min(by_height);
        let (drawn_w, drawn_h) = (gw * scale, gh * scale);
        Self {
            left: (width as f32 - drawn_w - LEAN * drawn_h) / 2.0,
            top: (height as f32 - drawn_h) / 2.0,
            scale,
            height: drawn_h,
        }
    }

    /// The glyph image's coverage under picture pixel `(x, y)`.
    fn coverage(&self, glyphs: &Image, x: f32, y: f32) -> f32 {
        let lean = LEAN * (self.top + self.height - y);
        let u = (x - self.left - lean) / self.scale;
        let v = (y - self.top) / self.scale;
        coverage_at(glyphs, u, v)
    }
}

/// Draws the glyphs and their shadow into BGRA `data`.
fn paint(data: &mut [u8], glyphs: &Image, width: u32, height: u32) {
    let place = Placement::fit(glyphs, width, height);
    let offset = height as f32 * SHADOW_OFFSET;
    for y in 0..height {
        for x in 0..width {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let fill = place.coverage(glyphs, px, py);
            let shadow = place.coverage(glyphs, px - offset, py - offset) * SHADOW_STRENGTH;
            let alpha = fill + shadow * (1.0 - fill);
            if alpha <= 0.0 {
                continue;
            }
            let grey = (255.0 * fill / alpha).round() as u8;
            let at = ((y * width + x) * 4) as usize;
            data[at..at + 4].copy_from_slice(&[grey, grey, grey, (alpha * 255.0).round() as u8]);
        }
    }
}

/// The alpha of the glyph image at the glyph pixel position `(u, v)`, blended between the four pixels around it; clear
/// outside the image.
fn coverage_at(glyphs: &Image, u: f32, v: f32) -> f32 {
    let (u, v) = (u - 0.5, v - 0.5);
    let (x0, y0) = (u.floor(), v.floor());
    let (fx, fy) = (u - x0, v - y0);
    let alpha = |x: f32, y: f32| -> f32 {
        if x < 0.0 || y < 0.0 || x >= glyphs.width as f32 || y >= glyphs.height as f32 {
            return 0.0;
        }
        f32::from(glyphs.rgba[((y as u32 * glyphs.width + x as u32) * 4 + 3) as usize]) / 255.0
    };
    let top = alpha(x0, y0) * (1.0 - fx) + alpha(x0 + 1.0, y0) * fx;
    let bottom = alpha(x0, y0 + 1.0) * (1.0 - fx) + alpha(x0 + 1.0, y0 + 1.0) * fx;
    top * (1.0 - fy) + bottom * fy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(texture: &Texture, x: u32, y: u32) -> u8 {
        texture.data[((y * texture.width + x) * 4 + 3) as usize]
    }

    #[test]
    fn an_empty_name_is_a_clear_picture_of_the_slots_shape() {
        let picture = texture("X", "  ", (64, 16));
        assert_eq!((picture.width, picture.height), (64, 16));
        assert_eq!(picture.data.len(), 64 * 16 * 4);
        assert!(picture.data.iter().all(|&byte| byte == 0));
    }

    #[test]
    fn a_name_is_drawn_inside_the_picture_and_leaves_its_edges_clear() {
        let picture = texture("Y", "Roadster 2011", (256, 64));
        let lit = |y: u32| (0..256).any(|x| alpha_at(&picture, x, y) > 0);
        assert!((0..64).any(lit), "the text shows");
        assert!(!lit(0) && !lit(63), "the top and bottom rows are clear");
    }

    #[test]
    fn the_picture_is_found_by_the_hash_of_its_upper_case_name() {
        assert_eq!(texture("name_one", "A", (8, 8)).name_hash, fe_hash_upper("NAME_ONE"));
    }

    #[test]
    fn a_logo_is_fitted_to_the_slot_without_stretching() {
        // A 4 by 2 white logo in an 8 by 8 slot becomes 8 by 4 in the middle rows.
        let logo = logo("L", (4, 2, &[255; 4 * 2 * 4]), (8, 8));
        assert_eq!((logo.width, logo.height), (8, 8));
        assert_eq!(alpha_at(&logo, 4, 0), 0, "the top row is clear");
        assert_eq!(alpha_at(&logo, 4, 7), 0, "the bottom row is clear");
        assert_eq!(alpha_at(&logo, 4, 4), 255, "the middle is the logo");
        assert_eq!(&logo.data[(4 * 8 + 4) * 4..][..3], &[255, 255, 255], "white stays white");
    }
}
