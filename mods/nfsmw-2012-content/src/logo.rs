//! A brand logo on the canvas the game's manufacturer slot expects: cut to its visible part, scaled to fit with a margin,
//! centred on a transparent 2:1 picture, and written as an 8-bit RGBA PNG.

use anyhow::{Context, Result};

/// Width and height of the canvas the logo is drawn on: 2:1, twice the slot's size, under the 600 x 300 the game cuts down.
pub const CANVAS: (usize, usize) = (300, 150);
/// Share of each canvas side left empty around the logo.
const MARGIN: f32 = 0.06;
/// A logo is never enlarged more than this: a 128 pixel picture blown up further is only blur.
const MAX_ENLARGE: f32 = 2.0;
/// Alpha at or below this is empty space when the visible part is measured.
const EMPTY_ALPHA: u8 = 8;

/// The smallest box (left, top, right, bottom, the last two exclusive) that holds every pixel with an alpha above
/// [`EMPTY_ALPHA`], or `None` for a picture with none.
fn visible_box(rgba: &[u8], width: usize, height: usize) -> Option<(usize, usize, usize, usize)> {
    let mut found: Option<(usize, usize, usize, usize)> = None;
    for y in 0..height {
        for x in 0..width {
            if rgba[(y * width + x) * 4 + 3] <= EMPTY_ALPHA {
                continue;
            }
            let (left, top, right, bottom) = found.unwrap_or((x, y, x + 1, y + 1));
            found = Some((left.min(x), top.min(y), right.max(x + 1), bottom.max(y + 1)));
        }
    }
    found
}

/// For each output sample the input samples and their weights: a triangle filter as wide as the step between samples when
/// shrinking, one sample either side when enlarging.
fn weights(input: usize, output: usize) -> Vec<Vec<(usize, f32)>> {
    let scale = input as f32 / output as f32;
    let support = scale.max(1.0);
    (0..output)
        .map(|out| {
            let centre = (out as f32 + 0.5) * scale;
            let first = (centre - support).floor().max(0.0) as usize;
            let last = ((centre + support).ceil() as usize).min(input);
            let mut taps: Vec<(usize, f32)> = (first..last)
                .map(|at| (at, (1.0 - ((at as f32 + 0.5 - centre) / support).abs()).max(0.0)))
                .filter(|&(_, weight)| weight > 0.0)
                .collect();
            let total: f32 = taps.iter().map(|&(_, weight)| weight).sum();
            for tap in &mut taps {
                tap.1 /= total;
            }
            taps
        })
        .collect()
}

/// `pixels` (premultiplied RGBA, `width` x `height`) resampled to `new_width` x `new_height`.
fn resample(pixels: &[f32], width: usize, height: usize, new_width: usize, new_height: usize) -> Vec<f32> {
    let across = weights(width, new_width);
    let mut wide = vec![0.0f32; new_width * height * 4];
    for y in 0..height {
        for (x, taps) in across.iter().enumerate() {
            for &(at, weight) in taps {
                for c in 0..4 {
                    wide[(y * new_width + x) * 4 + c] += pixels[(y * width + at) * 4 + c] * weight;
                }
            }
        }
    }
    let down = weights(height, new_height);
    let mut out = vec![0.0f32; new_width * new_height * 4];
    for (y, taps) in down.iter().enumerate() {
        for x in 0..new_width {
            for &(at, weight) in taps {
                for c in 0..4 {
                    out[(y * new_width + x) * 4 + c] += wide[(at * new_width + x) * 4 + c] * weight;
                }
            }
        }
    }
    out
}

/// The logo in `rgba` (R, G, B, A, `width` x `height`) on a [`CANVAS`] of RGBA bytes: its visible part scaled to fit inside
/// the margins, centred, with the rest transparent. `None` for a picture that is empty or whose size does not match its
/// data.
pub fn on_canvas(rgba: &[u8], width: usize, height: usize) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || rgba.len() != width * height * 4 {
        return None;
    }
    let (left, top, right, bottom) = visible_box(rgba, width, height)?;
    let (cut_width, cut_height) = (right - left, bottom - top);
    let mut cut = Vec::with_capacity(cut_width * cut_height * 4);
    for y in top..bottom {
        for x in left..right {
            let p = &rgba[(y * width + x) * 4..][..4];
            let alpha = f32::from(p[3]) / 255.0;
            cut.extend([f32::from(p[0]) * alpha, f32::from(p[1]) * alpha, f32::from(p[2]) * alpha, f32::from(p[3])]);
        }
    }
    let (canvas_width, canvas_height) = CANVAS;
    let room = (canvas_width as f32 * (1.0 - 2.0 * MARGIN), canvas_height as f32 * (1.0 - 2.0 * MARGIN));
    let scale = (room.0 / cut_width as f32).min(room.1 / cut_height as f32).min(MAX_ENLARGE);
    let new_width = ((cut_width as f32 * scale).round() as usize).clamp(1, canvas_width);
    let new_height = ((cut_height as f32 * scale).round() as usize).clamp(1, canvas_height);
    let scaled = resample(&cut, cut_width, cut_height, new_width, new_height);
    let (x0, y0) = ((canvas_width - new_width) / 2, (canvas_height - new_height) / 2);
    let mut canvas = vec![0u8; canvas_width * canvas_height * 4];
    for y in 0..new_height {
        for x in 0..new_width {
            let p = &scaled[(y * new_width + x) * 4..][..4];
            let alpha = p[3];
            let straight = |value: f32| {
                if alpha <= 0.0 {
                    return 0;
                }
                (value * 255.0 / alpha).round().clamp(0.0, 255.0) as u8
            };
            canvas[((y0 + y) * canvas_width + x0 + x) * 4..][..4].copy_from_slice(&[
                straight(p[0]),
                straight(p[1]),
                straight(p[2]),
                alpha.round().clamp(0.0, 255.0) as u8,
            ]);
        }
    }
    Some(canvas)
}

/// `rgba` (`width` x `height`) as an 8-bit RGBA PNG.
pub fn encode_png(rgba: &[u8], width: usize, height: usize) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().context("writing the PNG header")?;
    writer.write_image_data(rgba).context("writing the PNG pixels")?;
    writer.finish().context("finishing the PNG")?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `width` x `height` picture, transparent except the box (left, top, right, bottom) filled with `colour`.
    fn picture(width: usize, height: usize, filled: (usize, usize, usize, usize), colour: [u8; 4]) -> Vec<u8> {
        let mut rgba = vec![0u8; width * height * 4];
        for y in filled.1..filled.3 {
            for x in filled.0..filled.2 {
                rgba[(y * width + x) * 4..][..4].copy_from_slice(&colour);
            }
        }
        rgba
    }

    /// The box of visible pixels of a canvas.
    fn drawn(canvas: &[u8]) -> (usize, usize, usize, usize) {
        visible_box(canvas, CANVAS.0, CANVAS.1).unwrap()
    }

    #[test]
    fn a_square_logo_is_cut_scaled_to_the_height_and_centred() {
        let source = picture(128, 128, (30, 20, 90, 80), [200, 10, 10, 255]);
        let canvas = on_canvas(&source, 128, 128).unwrap();
        assert_eq!(canvas.len(), CANVAS.0 * CANVAS.1 * 4);
        let (left, top, right, bottom) = drawn(&canvas);
        assert_eq!((right - left, bottom - top), (120, 120), "a 60 pixel square enlarged 2x at most");
        assert!(left.abs_diff(CANVAS.0 - right) <= 1 && top.abs_diff(CANVAS.1 - bottom) <= 1, "centred");
        let middle = &canvas[((CANVAS.1 / 2) * CANVAS.0 + CANVAS.0 / 2) * 4..][..4];
        assert_eq!(middle, [200, 10, 10, 255], "the colour survives, un-premultiplied");
    }

    #[test]
    fn a_wide_logo_fits_the_width_and_is_not_enlarged_past_the_limit() {
        let wide = picture(256, 128, (0, 60, 256, 70), [255, 255, 255, 255]);
        let (left, top, right, bottom) = drawn(&on_canvas(&wide, 256, 128).unwrap());
        assert_eq!(right - left, 264, "300 less the margins");
        assert_eq!(bottom - top, 10);
        let thin = picture(64, 32, (0, 0, 64, 16), [1, 2, 3, 255]);
        let (left, _, right, _) = drawn(&on_canvas(&thin, 64, 32).unwrap());
        assert_eq!(right - left, 128, "64 pixels enlarged 2x at most");
    }

    #[test]
    fn a_large_logo_is_shrunk_and_stays_inside_the_margins() {
        let big = picture(1024, 1024, (0, 0, 1024, 1024), [9, 9, 9, 255]);
        let (left, top, right, bottom) = drawn(&on_canvas(&big, 1024, 1024).unwrap());
        assert_eq!((bottom - top, right - left), (132, 132));
        assert!(top >= 9 && bottom <= 141);
    }

    #[test]
    fn an_empty_or_mismatched_picture_gives_nothing() {
        assert!(on_canvas(&vec![0; 16 * 16 * 4], 16, 16).is_none());
        assert!(on_canvas(&[255; 8], 4, 4).is_none());
        assert!(on_canvas(&[], 0, 0).is_none());
    }

    #[test]
    fn half_transparent_edges_keep_their_colour() {
        let source = picture(8, 8, (0, 0, 8, 8), [100, 150, 200, 128]);
        let canvas = on_canvas(&source, 8, 8).unwrap();
        let (left, top, ..) = drawn(&canvas);
        let inside = &canvas[((top + 8) * CANVAS.0 + left + 8) * 4..][..4];
        assert!(inside[0].abs_diff(100) <= 1 && inside[1].abs_diff(150) <= 1 && inside[2].abs_diff(200) <= 1);
        assert!(inside[3].abs_diff(128) <= 2);
    }

    #[test]
    fn the_png_decodes_back_to_the_canvas() {
        let canvas = on_canvas(&picture(32, 32, (4, 4, 28, 28), [5, 6, 7, 255]), 32, 32).unwrap();
        let bytes = encode_png(&canvas, CANVAS.0, CANVAS.1).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().unwrap();
        let mut back = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut back).unwrap();
        assert_eq!((info.width as usize, info.height as usize), CANVAS);
        assert_eq!((info.color_type, info.bit_depth), (png::ColorType::Rgba, png::BitDepth::Eight));
        assert_eq!(back, canvas);
    }
}
