//! Multi images: a texture drawn through a second texture (the mask) that the game rotates about a pivot, which is
//! how the nitrous bar and the tachometer redline fill. The mask's alpha scales the alpha of the picture.
//! Rules and evidence: `docs/specs/feng-runtime.md` section 6 (the blend itself is inferred from the textures).

use crate::ui::assets::Image;

/// The picture with its alpha multiplied by the mask, whose sampling coordinates are rotated by `degrees` about
/// `pivot` (fractions of the texture; `[0.5, 0.5]` is the centre). `window` is the part of the mask texture that
/// covers the picture, `[u0, v0, u1, v1]`: `[0, 0, 1, 1]` for all of it, a window of the picture's size that
/// moves about the texture for the minimap. Returns straight (not premultiplied) RGBA of the picture's size.
/// Where the mask lies outside its texture the alpha is zero.
pub fn compose(base: &Image, mask: &Image, pivot: [f32; 2], degrees: f32, window: [f32; 4]) -> Image {
    let (bw, bh) = (base.width as usize, base.height as usize);
    if bw == 0 || bh == 0 {
        return Image { width: base.width, height: base.height, rgba: base.rgba.clone(), blend: base.blend };
    }
    let (mw, mh) = (mask.width as f32, mask.height as f32);
    let (sin, cos) = degrees.to_radians().sin_cos();
    // The offset of the pivot from the centre, in mask pixels.
    let (px, py) = (mw * pivot[0] - mw * 0.5, mh * pivot[1] - mh * 0.5);
    let (cx, cy) = (mw * 0.5, mh * 0.5);
    let mut rgba = base.rgba.clone();
    // The x coordinate is independent of the row. Keep the original operation order for exact rounding.
    let columns: Vec<f32> = (0..bw)
        .map(|x| {
            let u = window[0] + (x as f32 + 0.5) / bw as f32 * (window[2] - window[0]);
            u * mw - cx - px
        })
        .collect();
    if degrees == 0.0 {
        compose_unrotated(&mut rgba, mask, &columns, bh, [px, py, cx, cy], window);
        return Image { width: base.width, height: base.height, rgba, blend: base.blend };
    }
    for y in 0..bh {
        let v = window[1] + (y as f32 + 0.5) / bh as f32 * (window[3] - window[1]);
        let sy = v * mh - cy - py;
        for (x, &sx) in columns.iter().enumerate() {
            let qx = sx * cos + sy * sin + px + cx;
            let qy = sy * cos - sx * sin + py + cy;
            let a = &mut rgba[(y * bw + x) * 4 + 3];
            *a = (f32::from(*a) * alpha_at(mask, qx, qy) + 0.5) as u8;
        }
    }
    Image { width: base.width, height: base.height, rgba, blend: base.blend }
}

/// Texel indices and interpolation weight for one sampling axis, unchanged along a row/column.
#[derive(Clone, Copy)]
struct Axis {
    first: usize,
    next: usize,
    weight: f32,
}

fn axis(position: f32, extent: usize) -> Option<Axis> {
    if !(0.0..=extent as f32).contains(&position) || extent == 0 {
        return None;
    }
    let fraction = position - 0.5;
    let first = fraction.floor();
    let weight = fraction - first;
    let clamp = |index: i32| index.clamp(0, extent as i32 - 1) as usize;
    Some(Axis { first: clamp(first as i32), next: clamp(first as i32 + 1), weight })
}

/// A sliding unrotated mask (the minimap): factor the bilinear coordinates by axis, skip fully empty rows.
/// The channel arithmetic and sampling at texture edges match `alpha_at` exactly.
fn compose_unrotated(
    rgba: &mut [u8],
    mask: &Image,
    columns: &[f32],
    height: usize,
    [px, py, cx, cy]: [f32; 4],
    window: [f32; 4],
) {
    let width = columns.len();
    let (mw, mh) = (mask.width as usize, mask.height as usize);
    let columns: Vec<Option<Axis>> = columns.iter().map(|sx| axis(sx + px + cx, mw)).collect();
    for (y, row) in rgba.chunks_exact_mut(width * 4).enumerate() {
        let v = window[1] + (y as f32 + 0.5) / height as f32 * (window[3] - window[1]);
        let sy = v * mh as f32 - cy - py;
        let Some(vertical) = axis(sy + py + cy, mh) else {
            for pixel in row.as_chunks_mut::<4>().0 {
                pixel[3] = 0;
            }
            continue;
        };
        let (top, bottom) = (vertical.first * mw, vertical.next * mw);
        for (pixel, horizontal) in row.as_chunks_mut::<4>().0.iter_mut().zip(&columns) {
            let Some(horizontal) = horizontal else {
                pixel[3] = 0;
                continue;
            };
            let alpha = |index: usize| f32::from(mask.rgba[index * 4 + 3]) / 255.0;
            let tx = horizontal.weight;
            let a = alpha(top + horizontal.first) * (1.0 - tx) + alpha(top + horizontal.next) * tx;
            let b = alpha(bottom + horizontal.first) * (1.0 - tx) + alpha(bottom + horizontal.next) * tx;
            let opacity = a * (1.0 - vertical.weight) + b * vertical.weight;
            pixel[3] = (f32::from(pixel[3]) * opacity + 0.5) as u8;
        }
    }
}

/// Multiplies the colour of every pixel by its alpha (for a picture that is added to the screen: what the mask
/// hides must add nothing).
pub fn premultiply(image: &mut Image) {
    for px in image.rgba.as_chunks_mut::<4>().0 {
        let a = u32::from(px[3]);
        for c in &mut px[..3] {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
}

/// The mask alpha (0..1) at a position in mask pixels, bilinear between texel centres; zero outside the texture.
fn alpha_at(mask: &Image, x: f32, y: f32) -> f32 {
    let (w, h) = (mask.width as i32, mask.height as i32);
    if w == 0 || h == 0 {
        return 0.0;
    }
    if x < 0.0 || y < 0.0 || x > w as f32 || y > h as f32 {
        return 0.0;
    }
    let (fx, fy) = (x - 0.5, y - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let texel = |ix: i32, iy: i32| {
        let (ix, iy) = (ix.clamp(0, w - 1), iy.clamp(0, h - 1));
        f32::from(mask.rgba[(iy as usize * w as usize + ix as usize) * 4 + 3]) / 255.0
    };
    let (x0, y0) = (x0 as i32, y0 as i32);
    let top = texel(x0, y0) * (1.0 - tx) + texel(x0 + 1, y0) * tx;
    let bottom = texel(x0, y0 + 1) * (1.0 - tx) + texel(x0 + 1, y0 + 1) * tx;
    top * (1.0 - ty) + bottom * ty
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHOLE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

    /// A `size` x `size` image whose alpha is 255 where `inside(x, y)` and 0 elsewhere.
    fn image(size: u32, inside: impl Fn(u32, u32) -> bool) -> Image {
        let mut rgba = Vec::new();
        for y in 0..size {
            for x in 0..size {
                rgba.extend_from_slice(&[10, 20, 30, if inside(x, y) { 255 } else { 0 }]);
            }
        }
        Image { width: size, height: size, rgba, blend: 1 }
    }

    fn alpha(image: &Image, x: u32, y: u32) -> u8 {
        image.rgba[((y * image.width + x) * 4 + 3) as usize]
    }

    #[test]
    fn an_unrotated_mask_keeps_what_both_cover() {
        let base = image(16, |_, _| true);
        let mask = image(16, |x, _| x < 8);
        let out = compose(&base, &mask, [0.5, 0.5], 0.0, WHOLE);
        assert_eq!((alpha(&out, 2, 5), alpha(&out, 13, 5)), (255, 0));
        assert_eq!(&out.rgba[..3], &[10, 20, 30], "the colour is the picture's");
    }

    #[test]
    fn a_half_turn_swaps_the_halves() {
        let base = image(16, |_, _| true);
        let mask = image(16, |x, _| x < 8);
        let out = compose(&base, &mask, [0.5, 0.5], 180.0, WHOLE);
        assert_eq!((alpha(&out, 2, 5), alpha(&out, 13, 5)), (0, 255));
    }

    #[test]
    fn a_quarter_turn_moves_the_left_half_to_the_top_or_bottom() {
        let base = image(16, |_, _| true);
        let mask = image(16, |x, _| x < 8);
        let out = compose(&base, &mask, [0.5, 0.5], 90.0, WHOLE);
        // Rows in the covered half are whole; the other half is empty. (Which half is the rotation's sign.)
        let rows = (alpha(&out, 8, 2), alpha(&out, 8, 13));
        assert!(rows == (255, 0) || rows == (0, 255), "{rows:?}");
        assert_eq!(alpha(&out, 2, 2), alpha(&out, 13, 2), "a horizontal band");
    }

    #[test]
    fn a_window_moves_the_mask_over_the_picture() {
        let base = image(16, |_, _| true);
        // A mask that covers the left half of its texture.
        let mask = image(16, |x, _| x < 8);
        // The picture sees the mask's left part (u 0 to 0.4): all of it is covered.
        let left = compose(&base, &mask, [0.5, 0.5], 0.0, [0.0, 0.0, 0.4, 1.0]);
        assert!(left.rgba.chunks(4).all(|p| p[3] == 255));
        // Moved right by one picture width: the picture sees the mask's right half and what is past it.
        let right = compose(&base, &mask, [0.5, 0.5], 0.0, [0.5, 0.0, 1.5, 1.0]);
        assert!(right.rgba.chunks(4).all(|p| p[3] == 0));
        // Moved left by half a picture width: the right half of the picture sees the mask's left half.
        let back = compose(&base, &mask, [0.5, 0.5], 0.0, [-0.5, 0.0, 0.5, 1.0]);
        assert_eq!((alpha(&back, 2, 5), alpha(&back, 13, 5)), (0, 255));
    }

    #[test]
    fn premultiplying_removes_the_colour_the_mask_hides() {
        let base = image(16, |_, _| true);
        let mask = image(16, |x, _| x < 8);
        let mut out = compose(&base, &mask, [0.5, 0.5], 0.0, WHOLE);
        premultiply(&mut out);
        assert_eq!(&out.rgba[2 * 4..2 * 4 + 4], &[10, 20, 30, 255], "covered pixels keep their colour");
        assert_eq!(&out.rgba[13 * 4..13 * 4 + 4], &[0, 0, 0, 0], "hidden pixels add nothing");
    }

    #[test]
    fn a_picture_without_alpha_stays_empty() {
        let base = image(8, |_, _| false);
        let mask = image(8, |_, _| true);
        assert!(compose(&base, &mask, [0.5, 0.5], 33.0, WHOLE).rgba.chunks(4).all(|p| p[3] == 0));
    }

    #[test]
    fn scrolling_a_disc_over_four_tiles_has_no_mask_seam() {
        let base = image(32, |_, _| true);
        let mask = image(64, |x, y| {
            let (x, y) = (x as f32 + 0.5 - 32.0, y as f32 + 0.5 - 32.0);
            x * x + y * y < 31.0 * 31.0
        });
        let whole = image(64, |_, _| true);
        // Compare each independently composed tile with the same disc drawn across one continuous picture.
        for offset in [[0.0, 0.0], [0.25, -0.25], [-0.49, 0.49], [0.5, 0.5]] {
            let reference = compose(
                &whole,
                &mask,
                [0.5, 0.5],
                0.0,
                [-0.5 - offset[0], -0.5 - offset[1], 1.5 - offset[0], 1.5 - offset[1]],
            );
            for row in 0..2 {
                for column in 0..2 {
                    let left = column as f32 - 0.5 - offset[0];
                    let top = row as f32 - 0.5 - offset[1];
                    let tile = compose(&base, &mask, [0.5, 0.5], 0.0, [left, top, left + 1.0, top + 1.0]);
                    for y in 0..32 {
                        for x in 0..32 {
                            let want = alpha(&reference, column * 32 + x, row * 32 + y);
                            assert!(
                                alpha(&tile, x, y).abs_diff(want) <= 1,
                                "mask discontinuity at tile ({column},{row}), pixel ({x},{y}), offset {offset:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// The pre-optimization compositor is an exact-output oracle, independent of the row/axis fast paths.
    fn reference(base: &Image, mask: &Image, pivot: [f32; 2], degrees: f32, window: [f32; 4]) -> Vec<u8> {
        let (bw, bh) = (base.width as usize, base.height as usize);
        let (mw, mh) = (mask.width as f32, mask.height as f32);
        let (sin, cos) = degrees.to_radians().sin_cos();
        let (px, py) = (mw * pivot[0] - mw * 0.5, mh * pivot[1] - mh * 0.5);
        let (cx, cy) = (mw * 0.5, mh * 0.5);
        let mut rgba = base.rgba.clone();
        for y in 0..bh {
            for x in 0..bw {
                let u = window[0] + (x as f32 + 0.5) / bw as f32 * (window[2] - window[0]);
                let v = window[1] + (y as f32 + 0.5) / bh as f32 * (window[3] - window[1]);
                let sx = u * mw - cx - px;
                let sy = v * mh - cy - py;
                let qx = sx * cos + sy * sin + px + cx;
                let qy = sy * cos - sx * sin + py + cy;
                let a = &mut rgba[(y * bw + x) * 4 + 3];
                *a = (f32::from(*a) * alpha_at(mask, qx, qy) + 0.5) as u8;
            }
        }
        rgba
    }

    #[test]
    fn optimized_masks_match_the_previous_pixels_exactly() {
        let mut mask = image(64, |_, _| true);
        mask.height = 61;
        mask.rgba.truncate((mask.width * mask.height * 4) as usize);
        for (i, p) in mask.rgba.chunks_exact_mut(4).enumerate() {
            p[3] = ((i * 37) % 256) as u8;
        }
        for size in [2, 4, 31, 128, 512] {
            let mut base = image(size, |_, _| true);
            for (i, p) in base.rgba.chunks_exact_mut(4).enumerate() {
                p[3] = ((i * 53) % 256) as u8;
            }
            for degrees in [0.0, -0.0, 33.25, -90.0, 180.0] {
                for pivot in [[0.5, 0.5], [0.13, 0.91]] {
                    for window in [WHOLE, [-0.75, -0.25, 0.25, 0.75], [0.83, 1.31, -0.27, -0.17]] {
                        let actual = compose(&base, &mask, pivot, degrees, window);
                        assert_eq!(
                            actual.rgba,
                            reference(&base, &mask, pivot, degrees, window),
                            "changed pixels: size {size}, angle {degrees}, pivot {pivot:?}, window {window:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn empty_images_and_masks_do_not_panic_or_draw_unclipped_pixels() {
        let empty = image(0, |_, _| true);
        let base = image(16, |_, _| true);
        assert!(compose(&empty, &base, [0.5, 0.5], 0.0, WHOLE).rgba.is_empty());
        for degrees in [0.0, 33.0] {
            let out = compose(&base, &empty, [0.5, 0.5], degrees, WHOLE);
            assert!(out.rgba.chunks_exact(4).all(|pixel| pixel[3] == 0));
        }
    }
}
