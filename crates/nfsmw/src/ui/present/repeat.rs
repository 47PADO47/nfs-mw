//! Bounded texture repetition for authored FEng image UVs; see docs/specs/feng-image-repeat.md.

use crate::ui::assets::Image;

const MAX_QUADS: usize = 4096;

#[derive(Debug)]
pub struct Tile {
    /// Rectangle in the source unit quad, from 0 to 1.
    pub rect: [f32; 4],
    pub uv: [f32; 4],
}

/// The axes a texture repeats on: its pack record's `TilableUV`, bit 0 across and bit 1 down. An axis that does not
/// repeat clamps to the texture's edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tiling {
    pub across: bool,
    pub down: bool,
}

impl Tiling {
    pub fn from_byte(tilable_uv: u8) -> Self {
        Self { across: tilable_uv & 1 != 0, down: tilable_uv & 2 != 0 }
    }
}

/// None keeps the ordinary single-quad path, whose sampler clamps to the edge: a rectangle inside the unit square, one
/// that leaves it only on axes the texture does not repeat on, and malformed or unreasonably large rectangles.
pub fn tiles(uv: [f32; 4], tiling: Tiling) -> Option<Vec<Tile>> {
    if !uv.iter().all(|v| v.is_finite()) {
        return None;
    }
    let leaves = |a: f32, b: f32| !(0.0..=1.0).contains(&a) || !(0.0..=1.0).contains(&b);
    if !(tiling.across && leaves(uv[0], uv[2])) && !(tiling.down && leaves(uv[1], uv[3])) {
        return None;
    }
    let x = spans(uv[0], uv[2], tiling.across)?;
    let y = spans(uv[1], uv[3], tiling.down)?;
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

/// The spans of one axis: one per tile when the texture repeats on it, else the whole range with its coordinates kept, for
/// the edge to clamp them.
fn spans(start: f32, end: f32, repeats: bool) -> Option<Vec<[f32; 4]>> {
    if !repeats {
        return Some(vec![[0.0, 1.0, start, end]]);
    }
    segments(start, end)
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

/// One border texel around the image for the filter: the opposite edge supplies it on an axis the texture repeats on (the
/// diagonal corners too), the same edge on an axis it clamps on.
pub fn pad(image: &Image, tiling: Tiling) -> Image {
    let (w, h) = (image.width as usize, image.height as usize);
    let border = |at: usize, len: usize, repeats: bool| match repeats {
        true => (at + len - 1) % len,
        false => at.saturating_sub(1).min(len - 1),
    };
    let mut rgba = vec![0; (w + 2) * (h + 2) * 4];
    for y in 0..h + 2 {
        for x in 0..w + 2 {
            let src = (border(y, h, tiling.down) * w + border(x, w, tiling.across)) * 4;
            let dst = (y * (w + 2) + x) * 4;
            rgba[dst..dst + 4].copy_from_slice(&image.rgba[src..src + 4]);
        }
    }
    Image { width: image.width + 2, height: image.height + 2, rgba, blend: image.blend }
}

/// Map the original unit texture into the padded image's interior. On an axis that clamps, coordinates outside the unit
/// square land past the interior, where the sampler holds the edge texel.
pub fn padded_uv(uv: [f32; 4], size: [u32; 2]) -> [f32; 4] {
    std::array::from_fn(|i| {
        let dim = size[i % 2] as f32;
        (uv[i] * dim + 1.0) / (dim + 2.0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOTH: Tiling = Tiling { across: true, down: true };

    #[test]
    fn dialog_stripe_repeats_through_its_fractional_final_tiles() {
        let tiles = tiles([0.0, 0.0, 16.8125, 6.1875], BOTH).unwrap();
        assert_eq!(tiles.len(), 17 * 7);
        let last = tiles.last().unwrap();
        assert_eq!(last.rect[2..], [1.0, 1.0]);
        assert_eq!(last.uv, [0.0, 0.0, 0.8125, 0.1875]);
        let area: f32 = tiles.iter().map(|t| (t.rect[2] - t.rect[0]) * (t.rect[3] - t.rect[1])).sum();
        assert!((area - 1.0).abs() < 1e-5);
    }

    #[test]
    fn scrolling_and_reversed_uvs_keep_their_phase_and_direction() {
        let tiles = tiles([-0.25, 1.25, 1.25, -0.25], BOTH).unwrap();
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
            assert!(tiles(uv, BOTH).is_none(), "{uv:?}");
        }
    }

    #[test]
    fn filter_padding_wraps_both_axes_instead_of_clamping() {
        let image =
            Image { width: 2, height: 2, rgba: [1u8, 2, 3, 4].into_iter().flat_map(|v| [v; 4]).collect(), blend: 2 };
        let padded = pad(&image, BOTH);
        assert_eq!(
            padded.rgba.as_chunks::<4>().0.iter().map(|p| p[0]).collect::<Vec<_>>(),
            [4, 3, 4, 3, 2, 1, 2, 1, 4, 3, 4, 3, 2, 1, 2, 1]
        );
        assert_eq!(padded.blend, 2);
        assert_eq!(padded_uv([0.0, 0.0, 1.0, 1.0], [2, 2]), [0.25, 0.25, 0.75, 0.75]);
    }

    #[test]
    fn a_texture_that_does_not_repeat_keeps_the_single_quad_and_its_edge_clamp() {
        // The pursuit board's backing slid off its texture by its leave: the clamped edge rows are transparent.
        assert!(tiles([0.0, 0.59375, 1.0, 2.0], Tiling::default()).is_none());
        assert!(tiles([0.0, 0.0, 1.0, 2.0], Tiling { across: true, down: false }).is_none(), "repeats only across");
        assert!(tiles([0.0, 0.0, 1.0, 2.0], Tiling { across: false, down: true }).is_some());
    }

    #[test]
    fn a_texture_that_repeats_across_only_splits_across_and_keeps_its_rows_for_the_clamp() {
        let tiles = tiles([0.0, -0.5, 2.5, 1.25], Tiling { across: true, down: false }).unwrap();
        assert_eq!(tiles.len(), 3);
        assert!(tiles.iter().all(|t| t.rect[1] == 0.0 && t.rect[3] == 1.0 && t.uv[1] == -0.5 && t.uv[3] == 1.25));
        assert_eq!(tiles[2].uv[2], 0.5);
    }

    #[test]
    fn filter_padding_holds_the_edge_on_an_axis_that_clamps() {
        let image =
            Image { width: 2, height: 2, rgba: [1u8, 2, 3, 4].into_iter().flat_map(|v| [v; 4]).collect(), blend: 1 };
        let padded = pad(&image, Tiling { across: true, down: false });
        assert_eq!(
            padded.rgba.as_chunks::<4>().0.iter().map(|p| p[0]).collect::<Vec<_>>(),
            [2, 1, 2, 1, 2, 1, 2, 1, 4, 3, 4, 3, 4, 3, 4, 3]
        );
    }
}
