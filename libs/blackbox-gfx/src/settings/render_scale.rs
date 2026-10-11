//! Internal render resolution: the size the scene is drawn at, relative to the output surface.

/// Smallest accepted render scale (a quarter of the surface size per axis).
pub const MIN_RENDER_SCALE: f32 = 0.25;
/// Largest accepted render scale (supersampling at twice the surface size per axis).
pub const MAX_RENDER_SCALE: f32 = 2.0;
/// The scale that draws the scene at the surface's own size.
pub const DEFAULT_RENDER_SCALE: f32 = 1.0;

/// Clamp `scale` into [`MIN_RENDER_SCALE`]..=[`MAX_RENDER_SCALE`]. Non-finite values give 1.0.
pub fn clamp_render_scale(scale: f32) -> f32 {
    if !scale.is_finite() {
        return DEFAULT_RENDER_SCALE;
    }
    scale.clamp(MIN_RENDER_SCALE, MAX_RENDER_SCALE)
}

/// The internal render size for a `surface` size (width, height) at `scale`: each axis is scaled,
/// rounded to the nearest pixel and kept at least one pixel. The scale is clamped first.
pub fn scaled_size(surface: (u32, u32), scale: f32) -> (u32, u32) {
    let scale = f64::from(clamp_render_scale(scale));
    let axis = |v: u32| ((f64::from(v) * scale).round() as u32).max(1);
    (axis(surface.0), axis(surface.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_to_the_supported_range() {
        assert_eq!(clamp_render_scale(0.01), MIN_RENDER_SCALE);
        assert_eq!(clamp_render_scale(9.0), MAX_RENDER_SCALE);
        assert_eq!(clamp_render_scale(0.75), 0.75);
        assert_eq!(clamp_render_scale(f32::NAN), DEFAULT_RENDER_SCALE);
        assert_eq!(clamp_render_scale(f32::INFINITY), DEFAULT_RENDER_SCALE);
    }

    #[test]
    fn default_scale_keeps_the_surface_size() {
        for size in [(1, 1), (1280, 720), (1921, 1081), (7680, 4320)] {
            assert_eq!(scaled_size(size, DEFAULT_RENDER_SCALE), size);
        }
    }

    #[test]
    fn scales_each_axis_and_rounds() {
        assert_eq!(scaled_size((1920, 1080), 0.5), (960, 540));
        assert_eq!(scaled_size((1920, 1080), 2.0), (3840, 2160));
        assert_eq!(scaled_size((1001, 601), 0.5), (501, 301));
        assert_eq!(scaled_size((1920, 1080), 0.67), (1286, 724));
    }

    #[test]
    fn never_collapses_below_one_pixel_and_clamps_the_scale() {
        assert_eq!(scaled_size((1, 1), 0.25), (1, 1));
        assert_eq!(scaled_size((2, 3), 0.0), (1, 1));
        assert_eq!(scaled_size((100, 100), 50.0), (200, 200));
        assert_eq!(scaled_size((100, 100), -3.0), (25, 25));
    }
}
