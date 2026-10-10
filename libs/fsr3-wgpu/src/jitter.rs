//! Sub-pixel camera jitter: a Halton(2, 3) sequence, as FSR 3.1 expects.
//!
//! Each frame the application shifts the projection by a different sub-pixel offset and reports the
//! same offset to [`crate::Fsr3Inputs`]. The offsets are in render-resolution pixels, in `[-0.5, 0.5)`,
//! with +x to the right and +y down the screen (the image's row order).

use glam::{UVec2, Vec2};

/// The `index`-th element of the Halton sequence in `base` (index 1 is the first non-zero element).
pub fn halton(index: u32, base: u32) -> f32 {
    debug_assert!(base >= 2);
    let mut f = 1.0_f32;
    let mut result = 0.0_f32;
    let mut i = index;
    while i > 0 {
        f /= base as f32;
        result += f * (i % base) as f32;
        i /= base;
    }
    result
}

/// The number of distinct jitter positions AMD uses for a render width and display width:
/// `8 * (display / render)^2`, so 8 at native resolution and 32 at 2x.
pub fn phase_count(render_width: u32, display_width: u32) -> u32 {
    let ratio = display_width as f32 / render_width.max(1) as f32;
    ((8.0 * ratio * ratio) as u32).max(1)
}

/// The jitter offset for frame `index` in a cycle of `phase_count` positions, in render pixels.
pub fn offset(index: u32, phase_count: u32) -> Vec2 {
    let i = index % phase_count.max(1) + 1;
    Vec2::new(halton(i, 2) - 0.5, halton(i, 3) - 0.5)
}

/// The jitter offset for frame `index`, with the cycle length chosen for the two sizes.
pub fn offset_for_sizes(index: u32, render: UVec2, display: UVec2) -> Vec2 {
    offset(index, phase_count(render.x, display.x))
}

/// The translation to apply to a projection matrix for a pixel jitter, in normalised device
/// coordinates: `(2 * x / render_width, -2 * y / render_height)`.
///
/// Multiply the projection by a translation by this (x, y) (post-multiply the view-to-clip matrix with
/// `Mat4::from_translation`) so that clip-space x and y move by it after the projection.
pub fn projection_offset_ndc(jitter: Vec2, render_size: UVec2) -> Vec2 {
    Vec2::new(2.0 * jitter.x / render_size.x.max(1) as f32, -2.0 * jitter.y / render_size.y.max(1) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halton_base_two_is_the_van_der_corput_sequence() {
        let seq: Vec<f32> = (1..=7).map(|i| halton(i, 2)).collect();
        assert_eq!(seq, [0.5, 0.25, 0.75, 0.125, 0.625, 0.375, 0.875]);
    }

    #[test]
    fn halton_base_three_starts_with_thirds_and_ninths() {
        let seq: Vec<f32> = (1..=5).map(|i| halton(i, 3)).collect();
        let want = [1.0 / 3.0, 2.0 / 3.0, 1.0 / 9.0, 4.0 / 9.0, 7.0 / 9.0];
        for (a, b) in seq.iter().zip(want) {
            assert!((a - b).abs() < 1e-6, "{seq:?}");
        }
        assert_eq!(halton(0, 3), 0.0);
    }

    #[test]
    fn phase_count_follows_the_square_of_the_ratio() {
        assert_eq!(phase_count(1920, 1920), 8);
        assert_eq!(phase_count(1280, 1920), 18);
        assert_eq!(phase_count(960, 1920), 32);
        assert_eq!(phase_count(640, 1920), 72);
        assert_eq!(phase_count(0, 1920), 8 * 1920 * 1920);
    }

    #[test]
    fn offsets_cycle_and_stay_in_the_pixel() {
        let n = 18;
        for i in 0..3 * n {
            let o = offset(i, n);
            assert!((-0.5..0.5).contains(&o.x) && (-0.5..0.5).contains(&o.y), "{o:?}");
            assert_eq!(o, offset(i % n, n));
        }
        assert_eq!(offset(0, 8), Vec2::new(0.0, 1.0 / 3.0 - 0.5));
    }

    #[test]
    fn offsets_in_a_cycle_are_distinct() {
        let n = 32;
        for i in 0..n {
            for j in i + 1..n {
                assert_ne!(offset(i, n), offset(j, n), "{i} {j}");
            }
        }
    }

    #[test]
    fn projection_offset_moves_down_the_screen_for_positive_pixel_y() {
        let ndc = projection_offset_ndc(Vec2::new(0.5, 0.25), UVec2::new(100, 50));
        assert_eq!(ndc, Vec2::new(0.01, -0.01));
    }
}
