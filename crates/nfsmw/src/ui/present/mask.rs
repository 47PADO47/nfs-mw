//! Multi images: a texture drawn through a second texture (the mask) that the game rotates about a pivot, which is
//! how the nitrous bar and the tachometer redline fill. The mask's alpha scales the alpha of the picture.
//! Rules and evidence: `docs/specs/feng-runtime.md` section 6 (the blend itself is inferred from the textures).

use crate::ui::assets::Image;

/// The picture with its alpha multiplied by the mask, whose sampling coordinates are rotated by `degrees` about
/// `pivot` (fractions of the texture; `[0.5, 0.5]` is the centre). Returns straight (not premultiplied) RGBA of
/// the picture's size. Where the rotated mask leaves its texture the alpha is zero.
pub fn compose(base: &Image, mask: &Image, pivot: [f32; 2], degrees: f32) -> Image {
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
            let sx = (x as f32 + 0.5) / bw as f32 * mw - cx - px;
            let sy = (y as f32 + 0.5) / bh as f32 * mh - cy - py;
            let qx = sx * cos + sy * sin + px + cx;
            let qy = sy * cos - sx * sin + py + cy;
            let a = &mut rgba[(y * bw + x) * 4 + 3];
            *a = (f32::from(*a) * alpha_at(mask, qx, qy) + 0.5) as u8;
        }
    }
    Image { width: base.width, height: base.height, rgba, blend: base.blend }
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
        let out = compose(&base, &mask, [0.5, 0.5], 0.0);
        assert_eq!((alpha(&out, 2, 5), alpha(&out, 13, 5)), (255, 0));
        assert_eq!(&out.rgba[..3], &[10, 20, 30], "the colour is the picture's");
    }

    #[test]
    fn a_half_turn_swaps_the_halves() {
        let base = image(16, |_, _| true);
        let mask = image(16, |x, _| x < 8);
        let out = compose(&base, &mask, [0.5, 0.5], 180.0);
        assert_eq!((alpha(&out, 2, 5), alpha(&out, 13, 5)), (0, 255));
    }

    #[test]
    fn a_quarter_turn_moves_the_left_half_to_the_top_or_bottom() {
        let base = image(16, |_, _| true);
        let mask = image(16, |x, _| x < 8);
        let out = compose(&base, &mask, [0.5, 0.5], 90.0);
        // Rows in the covered half are whole; the other half is empty. (Which half is the rotation's sign.)
        let rows = (alpha(&out, 8, 2), alpha(&out, 8, 13));
        assert!(rows == (255, 0) || rows == (0, 255), "{rows:?}");
        assert_eq!(alpha(&out, 2, 2), alpha(&out, 13, 2), "a horizontal band");
    }

    #[test]
    fn a_picture_without_alpha_stays_empty() {
        let base = image(8, |_, _| false);
        let mask = image(8, |_, _| true);
        assert!(compose(&base, &mask, [0.5, 0.5], 33.0).rgba.chunks(4).all(|p| p[3] == 0));
    }
}
