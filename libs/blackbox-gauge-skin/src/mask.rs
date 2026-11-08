//! Sector masks: the arc's filled picture and the red zone's numbers are drawn through a mask that is `inside` a sector
//! of angles and `outside` it. Spec: `docs/specs/custom-hud-gauges.md` section 8.
//!
//! The angles are the needle's (degrees, clockwise on the screen, 0 pointing left). A pixel's direction from the picture's
//! centre is at angle `atan2(dy, dx) + 180` with y pointing down, so left is 0 and up is 90.

use crate::image::Image;

/// The angle of the direction `(dx, dy)` (y down) in the needle's convention, 0 to 360.
pub fn direction_angle(dx: f32, dy: f32) -> f32 {
    (dy.atan2(dx).to_degrees() + 180.0).rem_euclid(360.0)
}

/// Whether the direction `(dx, dy)` lies in the clockwise sweep from `start` to `end` (the two are put in order first). A
/// sweep of 360 degrees or more holds every direction.
pub fn in_sector(dx: f32, dy: f32, start: f32, end: f32) -> bool {
    let (first, last) = match start <= end {
        true => (start, end),
        false => (end, start),
    };
    let span = last - first;
    if span >= 360.0 {
        return true;
    }
    (direction_angle(dx, dy) - first).rem_euclid(360.0) <= span
}

/// The picture with its alpha multiplied by the mask: `inside_alpha` (0 to 255, as a share of 255) for pixels in the sector,
/// `outside_alpha` for the others. Colour is untouched.
pub fn masked(base: &Image, start: f32, end: f32, inside_alpha: u8, outside_alpha: u8) -> Image {
    let (width, height) = (base.width as usize, base.height as usize);
    let mut rgba = base.rgba.clone();
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 + 0.5 - width as f32 / 2.0;
            let dy = y as f32 + 0.5 - height as f32 / 2.0;
            let factor = match in_sector(dx, dy, start, end) {
                true => u32::from(inside_alpha),
                false => u32::from(outside_alpha),
            };
            let at = (y * width + x) * 4 + 3;
            rgba[at] = ((u32::from(rgba[at]) * factor + 127) / 255) as u8;
        }
    }
    Image { width: base.width, height: base.height, rgba }
}

/// An angle in whole steps of `step` degrees, so that a mask is rebuilt only when the angle moves a step.
pub fn quantise(angle: f32, step: f32) -> i32 {
    (angle / step).round() as i32
}

/// The angle of a step number made by [`quantise`].
pub fn dequantise(steps: i32, step: f32) -> f32 {
    steps as f32 * step
}

#[cfg(test)]
mod tests {
    use super::*;

    fn white(side: u32) -> Image {
        Image { width: side, height: side, rgba: vec![255; (side * side * 4) as usize] }
    }

    #[test]
    fn angles_run_clockwise_from_left() {
        assert!((direction_angle(-1.0, 0.0) - 0.0).abs() < 1e-3 || (direction_angle(-1.0, 0.0) - 360.0).abs() < 1e-3);
        assert!((direction_angle(0.0, -1.0) - 90.0).abs() < 1e-3, "up is 90");
        assert!((direction_angle(1.0, 0.0) - 180.0).abs() < 1e-3, "right is 180");
        assert!((direction_angle(0.0, 1.0) - 270.0).abs() < 1e-3, "down is 270");
    }

    #[test]
    fn a_sector_holds_the_directions_swept_clockwise() {
        // From the left, over the top, to the right: the upper half.
        assert!(in_sector(0.0, -1.0, 0.0, 180.0));
        assert!(in_sector(-1.0, -0.2, 0.0, 180.0));
        assert!(!in_sector(0.0, 1.0, 0.0, 180.0));
        // A needle's whole range, -30 to 210: everything but the bottom 120 degrees.
        assert!(in_sector(-1.0, 0.3, -30.0, 210.0), "just under the left is inside (angle 343)");
        assert!(in_sector(0.0, -1.0, -30.0, 210.0));
        assert!(!in_sector(0.0, 1.0, -30.0, 210.0), "straight down is in the gap");
    }

    #[test]
    fn the_two_ends_are_put_in_order_and_a_full_turn_holds_everything() {
        assert_eq!(in_sector(0.0, -1.0, 180.0, 0.0), in_sector(0.0, -1.0, 0.0, 180.0));
        assert!(in_sector(0.0, 1.0, -10.0, 400.0));
    }

    #[test]
    fn the_mask_scales_alpha_inside_and_outside_the_sector() {
        let picture = masked(&white(8), 0.0, 180.0, 255, 0);
        let alpha = |x: usize, y: usize| picture.rgba[(y * 8 + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 255, "top left is in the upper half");
        assert_eq!(alpha(7, 0), 255, "top right too");
        assert_eq!(alpha(0, 7), 0, "bottom left is outside");
        assert_eq!(alpha(7, 7), 0);
        assert_eq!(picture.rgba[0], 255, "colour is untouched");
    }

    #[test]
    fn the_outside_keeps_a_share_of_the_alpha() {
        // 0x3C is the outside value of the shipped arcs: 60 of 255.
        let picture = masked(&white(4), 0.0, 180.0, 255, 0x3C);
        let bottom = picture.rgba[(3 * 4) * 4 + 3];
        assert_eq!(bottom, 60);
    }

    #[test]
    fn angles_quantise_to_steps_and_back() {
        assert_eq!(quantise(61.2, 1.5), 41);
        assert!((dequantise(41, 1.5) - 61.5).abs() < 1e-5);
        assert_eq!(quantise(-30.0, 1.5), -20);
        assert_eq!(quantise(0.7, 1.5), 0);
    }
}
