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
    let (mw, mh) = (mask.width as f32, mask.height as f32);
    let (sin, cos) = degrees.to_radians().sin_cos();
    // The offset of the pivot from the centre, in mask pixels.
    let (px, py) = (mw * pivot[0] - mw * 0.5, mh * pivot[1] - mh * 0.5);
    let (cx, cy) = (mw * 0.5, mh * 0.5);
    let mut rgba = base.rgba.clone();
    for y in 0..bh {
        for x in 0..bw {
            // The same place in the mask texture, then through the rotation of the mask coordinates.
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
    Image { width: base.width, height: base.height, rgba, blend: base.blend }
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
}
