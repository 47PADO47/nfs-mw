//! Bounded texture repetition for authored FEng image UVs; see docs/specs/feng-image-repeat.md.

use crate::ui::assets::Image;

const MAX_QUADS: usize = 4096;

#[derive(Debug)]
pub struct Tile {
    /// Rectangle in the source unit quad, from 0 to 1.
    pub rect: [f32; 4],
    pub uv: [f32; 4],
}

/// None keeps the ordinary single-quad path, including malformed/unreasonably large rectangles.
pub fn tiles(uv: [f32; 4]) -> Option<Vec<Tile>> {
    if uv.iter().all(|v| (0.0..=1.0).contains(v)) || !uv.iter().all(|v| v.is_finite()) {
        return None;
    }
    let x = segments(uv[0], uv[2])?;
    let y = segments(uv[1], uv[3])?;
    if x.len().checked_mul(y.len())? > MAX_QUADS {
        return None;
    }
    let mut result = Vec::with_capacity(x.len() * y.len());
    for vy in &y {
        for vx in &x {
            result.push(Tile { rect: [vx[0], vy[0], vx[1], vy[1]], uv: [vx[2], vy[2], vx[3], vy[3]] });
        }
    }
    Some(result)
}

fn segments(start: f32, end: f32) -> Option<Vec<[f32; 4]>> {
    let (lo, hi) = (start.min(end).floor(), start.max(end).ceil());
    if hi - lo > MAX_QUADS as f32 || lo.abs().max(hi.abs()) > 1_000_000.0 {
        return None;
    }
    if start == end {
        return Some(vec![[0.0, 1.0, start.rem_euclid(1.0), start.rem_euclid(1.0)]]);
    }
    let mut cuts = vec![0.0, 1.0];
    for boundary in lo as i32..=hi as i32 {
        let at = (boundary as f32 - start) / (end - start);
        if at > 0.0 && at < 1.0 {
            cuts.push(at);
        }
    }
    cuts.sort_by(f32::total_cmp);
    Some(
        cuts.windows(2)
            .map(|w| {
                let base = (start + (end - start) * ((w[0] + w[1]) * 0.5)).floor();
                let local = |at: f32| (start + (end - start) * at - base).clamp(0.0, 1.0);
                [w[0], w[1], local(w[0]), local(w[1])]
            })
            .collect(),
    )
}

/// The opposite edge supplies each filter-border texel, including the diagonal corner samples.
pub fn pad(image: &Image) -> Image {
    let (w, h) = (image.width as usize, image.height as usize);
    let mut rgba = vec![0; (w + 2) * (h + 2) * 4];
    for y in 0..h + 2 {
        for x in 0..w + 2 {
            let src = (((y + h - 1) % h) * w + (x + w - 1) % w) * 4;
            let dst = (y * (w + 2) + x) * 4;
            rgba[dst..dst + 4].copy_from_slice(&image.rgba[src..src + 4]);
        }
    }
    Image { width: image.width + 2, height: image.height + 2, rgba, blend: image.blend }
}

/// Map the original unit texture into the wrapped image's unpadded interior.
pub fn padded_uv(uv: [f32; 4], size: [u32; 2]) -> [f32; 4] {
    std::array::from_fn(|i| {
        let dim = size[i % 2] as f32;
        (uv[i] * dim + 1.0) / (dim + 2.0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_stripe_repeats_through_its_fractional_final_tiles() {
        let tiles = tiles([0.0, 0.0, 16.8125, 6.1875]).unwrap();
        assert_eq!(tiles.len(), 17 * 7);
        let last = tiles.last().unwrap();
        assert_eq!(last.rect[2..], [1.0, 1.0]);
        assert_eq!(last.uv, [0.0, 0.0, 0.8125, 0.1875]);
        let area: f32 = tiles.iter().map(|t| (t.rect[2] - t.rect[0]) * (t.rect[3] - t.rect[1])).sum();
        assert!((area - 1.0).abs() < 1e-5);
    }

    #[test]
    fn scrolling_and_reversed_uvs_keep_their_phase_and_direction() {
        let tiles = tiles([-0.25, 1.25, 1.25, -0.25]).unwrap();
        assert_eq!(tiles.len(), 9);
        assert_eq!(tiles[0].uv, [0.75, 0.25, 1.0, 0.0]);
        assert_eq!(tiles[8].uv, [0.0, 1.0, 0.25, 0.75]);
        assert!(tiles.iter().all(|t| t.uv.iter().all(|v| (0.0..=1.0).contains(v))));
    }

    #[test]
    fn unit_atlas_uvs_stay_on_the_single_quad_path_and_bad_ranges_are_bounded() {
        for uv in [
            [0.1, 0.2, 0.8, 0.9],
            [1.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 4097.0, 1.0],
            [0.0, 0.0, 100.0, 100.0],
            [0.0, 0.0, f32::INFINITY, 1.0],
            [f32::NAN; 4],
        ] {
            assert!(tiles(uv).is_none(), "{uv:?}");
        }
    }

    #[test]
    fn filter_padding_wraps_both_axes_instead_of_clamping() {
        let image =
            Image { width: 2, height: 2, rgba: [1u8, 2, 3, 4].into_iter().flat_map(|v| [v; 4]).collect(), blend: 2 };
        let padded = pad(&image);
        assert_eq!(
            padded.rgba.as_chunks::<4>().0.iter().map(|p| p[0]).collect::<Vec<_>>(),
            [4, 3, 4, 3, 2, 1, 2, 1, 4, 3, 4, 3, 2, 1, 2, 1]
        );
        assert_eq!(padded.blend, 2);
        assert_eq!(padded_uv([0.0, 0.0, 1.0, 1.0], [2, 2]), [0.25, 0.25, 0.75, 0.75]);
    }
}
