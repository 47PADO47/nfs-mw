//! The quads of a bar in world space: the border, the lagging segment and the fill, laid along the camera's right
//! vector. Spec section 5. A bar is drawn as flat-colour quads, so the caller needs no texture coordinates.

use glam::Vec3;

use super::bar::Bar;
use super::params::BarParams;

/// Four corners in the order bottom-left, bottom-right, top-right, top-left, and a linear RGBA colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    pub corners: [Vec3; 4],
    pub rgba: [f32; 4],
}

/// The corners of a `width` by `height` rectangle centred on `centre`, its width along `right` and its height along
/// `up`. Order: bottom-left, bottom-right, top-right, top-left.
pub fn bar_corners(centre: Vec3, right: Vec3, up: Vec3, width: f32, height: f32) -> [Vec3; 4] {
    let half_w = right * (width * 0.5);
    let half_h = up * (height * 0.5);
    [centre - half_w - half_h, centre + half_w - half_h, centre + half_w + half_h, centre - half_w + half_h]
}

/// The corners of the strip between the fractions `from` and `to` of an inner area `inner_width` wide, measured
/// from its left end. The inner area is centred on `centre` and is `inner_height` tall.
pub fn span_corners(
    centre: Vec3,
    right: Vec3,
    up: Vec3,
    inner_width: f32,
    inner_height: f32,
    from: f32,
    to: f32,
) -> [Vec3; 4] {
    let left = centre - right * (inner_width * 0.5);
    let start = left + right * (inner_width * from);
    let end = left + right * (inner_width * to);
    let half_h = up * (inner_height * 0.5);
    [start - half_h, end - half_h, end + half_h, start + half_h]
}

/// The quads of one bar in draw order: border, then the lagging segment, then the fill, so the fill draws over the
/// lag where they meet. A segment that is empty is left out: the lag when `lag <= health`, the fill at 0 health.
pub fn segment_quads(bar: &Bar, params: &BarParams) -> Vec<Quad> {
    let up = Vec3::Z;
    let inner_w = params.width - 2.0 * params.border;
    let inner_h = params.height - 2.0 * params.border;
    let mut quads = vec![Quad {
        corners: bar_corners(bar.centre, bar.right, up, params.width, params.height),
        rgba: bar.border_rgba,
    }];
    if bar.lag > bar.health {
        quads.push(Quad {
            corners: span_corners(bar.centre, bar.right, up, inner_w, inner_h, bar.health, bar.lag),
            rgba: bar.lag_rgba,
        });
    }
    if bar.health > 0.0 {
        quads.push(Quad {
            corners: span_corners(bar.centre, bar.right, up, inner_w, inner_h, 0.0, bar.health),
            rgba: bar.fill_rgba,
        });
    }
    quads
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health_bar::approx;
    use crate::health_bar::bar::test_bar;

    fn assert_corners(actual: [Vec3; 4], expected: [Vec3; 4]) {
        for (a, e) in actual.iter().zip(expected.iter()) {
            assert!(approx(a.x, e.x) && approx(a.y, e.y) && approx(a.z, e.z), "{a:?} != {e:?}");
        }
    }

    #[test]
    fn bar_facing_plus_x_with_right_minus_y_has_expected_corners() {
        let corners = bar_corners(Vec3::ZERO, Vec3::NEG_Y, Vec3::Z, 2.0, 1.0);
        assert_corners(
            corners,
            [
                Vec3::new(0.0, 1.0, -0.5),
                Vec3::new(0.0, -1.0, -0.5),
                Vec3::new(0.0, -1.0, 0.5),
                Vec3::new(0.0, 1.0, 0.5),
            ],
        );
    }

    #[test]
    fn fill_and_lag_widths_follow_health_and_lag() {
        let params = BarParams::default();
        let bar = Bar { health: 0.5, lag: 0.75, right: Vec3::X, ..test_bar(1, 0.0) };
        let quads = segment_quads(&bar, &params);
        assert_eq!(quads.len(), 3, "border, lag, fill");
        let inner = params.width - 2.0 * params.border;
        // Lag runs from the fill end (0.5 of the inner width) to 0.75 of it; the fill from the left end to 0.5.
        let lag_x = (quads[1].corners[0].x, quads[1].corners[1].x);
        let fill_x = (quads[2].corners[0].x, quads[2].corners[1].x);
        assert!(approx(fill_x.0, -inner / 2.0));
        assert!(approx(fill_x.1, -inner / 2.0 + inner * 0.5));
        assert!(approx(lag_x.0, -inner / 2.0 + inner * 0.5));
        assert!(approx(lag_x.1, -inner / 2.0 + inner * 0.75));
    }

    #[test]
    fn empty_lag_and_zero_health_leave_their_quads_out() {
        let params = BarParams::default();
        let no_lag = Bar { health: 0.6, lag: 0.6, right: Vec3::X, ..test_bar(1, 0.0) };
        assert_eq!(segment_quads(&no_lag, &params).len(), 2, "border and fill");
        let dead = Bar { health: 0.0, lag: 0.0, right: Vec3::X, ..test_bar(1, 0.0) };
        assert_eq!(segment_quads(&dead, &params).len(), 1, "border only");
    }
}
