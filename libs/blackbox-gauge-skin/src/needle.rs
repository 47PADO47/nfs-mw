//! The needle: the angle a value turns it to and the shape of its picture. Spec: `docs/specs/custom-hud-gauges.md` section 5.
//!
//! Angles are degrees, clockwise on the screen, from the direction "left": 0 points left, 90 up, 180 right, 270 down.

/// The needle's angle for `value` on a scale that ends at `scale_value`: `min_angle` at 0 and `max_angle` at the end. A value
/// above the scale stops at its end; a negative one is floored at 0 unless `can_be_negative`. A scale of 0 or less gives
/// `min_angle`.
pub fn angle(min_angle: f32, max_angle: f32, scale_value: f32, value: f32, can_be_negative: bool) -> f32 {
    if scale_value <= 0.0 || !value.is_finite() {
        return min_angle;
    }
    let capped = value.min(scale_value);
    let capped = match can_be_negative {
        true => capped,
        false => capped.max(0.0),
    };
    min_angle + (max_angle - min_angle) / scale_value * capped
}

/// The needle's picture, in reference pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub width: f32,
    pub height: f32,
    /// The turning point's distance from the picture's left edge. Its distance from the top is half the height.
    pub pivot_x: f32,
}

/// The needle of a gauge `size` pixels wide: `size / 1.4` by `size / 2.8` times `scale`, turning about the point
/// `1 - min(centre_offset, 1)` of the way across it (0: its right end, 1: its left end).
pub fn shape(size: f32, scale: f32, centre_offset: f32) -> Shape {
    let width = size / 1.4 * scale;
    let height = size / 2.8 * scale;
    Shape { width, height, pivot_x: width * (1.0 - centre_offset.min(1.0)) }
}

/// Whether the near-red-line colour applies: `threshold` is above 0 and the red line (thousands of rpm) is less than
/// `threshold` above `value`. A gauge with no red line compares the value with itself, so any positive threshold applies.
pub fn near_redline(threshold: f32, value: f32, redline: Option<f32>) -> bool {
    if threshold <= 0.0 {
        return false;
    }
    let redline = redline.unwrap_or(value);
    redline - value < threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tachometer_sweeps_from_minus_30_to_210_over_10_thousand_rpm() {
        let at = |rpm_thousands: f32| angle(-30.0, 210.0, 10.0, rpm_thousands, true);
        assert_eq!(at(0.0), -30.0);
        assert_eq!(at(5.0), 90.0);
        assert_eq!(at(10.0), 210.0);
        assert!((at(3.8) - 61.2).abs() < 1e-3, "a needle at 3.8 stands about 61 degrees on, up and to the left");
    }

    #[test]
    fn values_beyond_the_scale_stop_at_its_ends() {
        assert_eq!(angle(-30.0, 210.0, 10.0, 14.0, true), 210.0);
        assert_eq!(
            angle(90.0, 210.0, 20.0, -5.0, true),
            90.0 + 120.0 / 20.0 * -5.0,
            "the boost can go below its start"
        );
        assert_eq!(angle(90.0, 210.0, 20.0, -5.0, false), 90.0);
        assert_eq!(angle(-30.0, 210.0, 0.0, 3.0, true), -30.0, "a scale of 0 has no sweep");
        assert_eq!(angle(-30.0, 210.0, 10.0, f32::NAN, true), -30.0);
    }

    #[test]
    fn the_needle_is_twice_as_wide_as_it_is_tall_and_turns_at_its_right_end() {
        let shape = shape(330.0, 1.0, 0.0);
        assert!((shape.width - 330.0 / 1.4).abs() < 1e-3);
        assert!((shape.height - 330.0 / 2.8).abs() < 1e-3);
        assert!((shape.width / shape.height - 2.0).abs() < 1e-5);
        assert_eq!(shape.pivot_x, shape.width);
    }

    #[test]
    fn the_centre_offset_moves_the_pivot_toward_the_left_end_and_stops_there() {
        assert!((super::shape(100.0, 1.0, 0.5).pivot_x - super::shape(100.0, 1.0, 0.0).width / 2.0).abs() < 1e-4);
        assert_eq!(super::shape(100.0, 1.0, 1.0).pivot_x, 0.0);
        assert_eq!(super::shape(100.0, 1.0, 3.0).pivot_x, 0.0, "an offset above 1 counts as 1");
        assert_eq!(super::shape(100.0, 2.0, 0.0).width, 2.0 * super::shape(100.0, 1.0, 0.0).width);
    }

    #[test]
    fn the_red_line_colour_needs_a_positive_threshold_and_a_near_red_line() {
        assert!(near_redline(0.5, 7.2, Some(7.5)));
        assert!(!near_redline(0.5, 6.0, Some(7.5)));
        assert!(!near_redline(0.0, 7.4, Some(7.5)), "a threshold of 0 never switches");
        assert!(near_redline(0.5, 7.4, None), "no red line: the value is compared with itself, so a threshold applies");
        assert!(near_redline(0.5, 8.0, Some(7.5)), "past the red line is near it");
    }
}
