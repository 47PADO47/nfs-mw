//! The Remastered tachometer's path in picture pixels: a quarter of an ellipse from the bottom left of the card, up its
//! left and over its top, then a straight run to the right along the top. The rev numbers sit along it, one thousand rpm
//! apart, and the arc is drawn along it. [`Path::at`] gives a point of it by the share of its length.

/// The ellipse: its centre, which is where the quarter ends and the top run begins, and its two radii.
const CENTRE: (f32, f32) = (448.0, 267.0);
const RX: f32 = 349.0;
const RY: f32 = 228.0;
/// Where the top run ends, along the top of the ellipse.
const TAIL_END_X: f32 = 541.0;
/// Samples along the quarter: enough that the lengths between them are almost straight.
const SAMPLES: usize = 400;

/// One point of the path: where it is, which way is in (toward the card's middle, a unit vector), and the share of the
/// path's length it is at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Point {
    pub x: f32,
    pub y: f32,
    pub inward: (f32, f32),
    pub share: f32,
}

/// The path with its lengths worked out.
pub(super) struct Path {
    points: Vec<Point>,
    length: f32,
}

impl Path {
    pub fn new() -> Self {
        let mut points = Vec::with_capacity(SAMPLES + 2);
        for k in 0..=SAMPLES {
            let t = std::f32::consts::FRAC_PI_2 * k as f32 / SAMPLES as f32;
            let (cos, sin) = (t.cos(), t.sin());
            let inward = normalised((cos / RX, sin / RY));
            points.push(Point { x: CENTRE.0 - RX * cos, y: CENTRE.1 - RY * sin, inward, share: 0.0 });
        }
        points.push(Point { x: TAIL_END_X, y: CENTRE.1 - RY, inward: (0.0, 1.0), share: 0.0 });
        let mut length = 0.0;
        let mut lengths = vec![0.0];
        for pair in points.windows(2) {
            length += ((pair[1].x - pair[0].x).powi(2) + (pair[1].y - pair[0].y).powi(2)).sqrt();
            lengths.push(length);
        }
        for (point, along) in points.iter_mut().zip(lengths) {
            point.share = along / length;
        }
        Self { points, length }
    }

    /// The path's length in picture pixels.
    pub fn length(&self) -> f32 {
        self.length
    }

    /// The point `share` of the way along the path (0 the start at the bottom left, 1 the end of the top run). A share
    /// outside 0 to 1 is clamped.
    pub fn at(&self, share: f32) -> Point {
        let share = share.clamp(0.0, 1.0);
        let next = self.points.partition_point(|p| p.share < share).clamp(1, self.points.len() - 1);
        let (a, b) = (self.points[next - 1], self.points[next]);
        let span = (b.share - a.share).max(f32::EPSILON);
        let t = ((share - a.share) / span).clamp(0.0, 1.0);
        let inward =
            normalised((a.inward.0 + (b.inward.0 - a.inward.0) * t, a.inward.1 + (b.inward.1 - a.inward.1) * t));
        Point { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t, inward, share }
    }
}

fn normalised((x, y): (f32, f32)) -> (f32, f32) {
    let length = (x * x + y * y).sqrt().max(f32::EPSILON);
    (x / length, y / length)
}

/// How far the card's lower left corner is cut: it leans out this many pixels to the left for each one it drops below the
/// arc's foot.
const FOOT_SLOPE: f32 = 1.36;
/// The card's right side leans: this many pixels further right for each one it rises, and where it stands at the bottom.
const RIGHT_SLOPE: f32 = 0.188;

/// The left edge of the dark card at picture height `y`: it follows the ellipse up to the top, and below the arc's foot it
/// leans out to the left.
pub(super) fn card_left(y: f32) -> f32 {
    if y > CENTRE.1 {
        return CENTRE.0 - RX - (y - CENTRE.1) * FOOT_SLOPE;
    }
    let up = ((CENTRE.1 - y) / RY).min(1.0);
    CENTRE.0 - RX * (1.0 - up * up).sqrt()
}

/// The right edge of the card at picture height `y`, given where it stands at `bottom`.
pub(super) fn card_right(y: f32, bottom: f32, edge: f32) -> f32 {
    edge + (bottom - y) * RIGHT_SLOPE
}

/// The picture height of the arc's foot, where the arc starts.
pub(super) const FOOT_Y: f32 = CENTRE.1;
/// The picture x of the arc's foot.
pub(super) const FOOT_X: f32 = CENTRE.0 - RX;

/// The top of the card: the ellipse's top.
pub(super) const TOP: f32 = CENTRE.1 - RY;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_runs_from_the_bottom_left_up_over_the_top_and_along_it() {
        let path = Path::new();
        let (start, end) = (path.at(0.0), path.at(1.0));
        assert!((start.x - (CENTRE.0 - RX)).abs() < 1e-3 && (start.y - CENTRE.1).abs() < 1e-3, "{start:?}");
        assert!((end.x - TAIL_END_X).abs() < 1e-3 && (end.y - (CENTRE.1 - RY)).abs() < 1e-3, "{end:?}");
        let top = path.at(0.5);
        assert!(top.y < CENTRE.1 && top.x > start.x, "half way is up and to the right");
        assert!(path.length() > 500.0 && path.length() < 600.0, "about the quarter and the run: {}", path.length());
    }

    #[test]
    fn the_inward_direction_points_into_the_card_and_is_a_unit_vector() {
        let path = Path::new();
        for share in [0.0, 0.2, 0.5, 0.7, 1.0] {
            let point = path.at(share);
            let length = (point.inward.0.powi(2) + point.inward.1.powi(2)).sqrt();
            assert!((length - 1.0).abs() < 1e-3);
            assert!(point.inward.0 >= -1e-4 && point.inward.1 >= -1e-4, "right or down at {share}: {point:?}");
        }
        assert!((path.at(1.0).inward.1 - 1.0).abs() < 1e-3, "the top run's inside is below it");
    }

    #[test]
    fn a_share_is_clamped_and_shares_increase_along_the_path() {
        let path = Path::new();
        assert_eq!(path.at(-1.0), path.at(0.0));
        assert_eq!(path.at(2.0), path.at(1.0));
        assert!(path.at(0.3).x < path.at(0.4).x);
    }

    #[test]
    fn the_card_follows_the_ellipse_above_its_centre_line_and_its_edge_below() {
        assert!(card_left(CENTRE.1 + 10.0) < CENTRE.0 - RX, "below the foot the corner leans out");
        assert!((card_left(CENTRE.1) - (CENTRE.0 - RX)).abs() < 1e-3);
        assert!(card_left(TOP) > card_left(TOP + 100.0), "narrower toward the top");
        assert!((card_left(TOP - 50.0) - CENTRE.0).abs() < 1e-3, "above the top it stays at the top's point");
        assert!(card_right(TOP, 289.0, 526.0) > card_right(289.0, 289.0, 526.0), "the right side leans out at the top");
    }
}
