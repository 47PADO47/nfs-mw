//! The clip region of a polyline: a circle, an ellipse or a rectangle the strokes stay inside, with an optional soft edge.
//!
//! A segment is clipped analytically: the part of its centre line inside the region is one interval along it, because
//! every region is convex. The stroke's own width is not clipped, so a clipped end can reach half the width past the
//! edge; with a fade band it is transparent there anyway.
//!
//! The fade measures how far a point is inside the edge. For a circle and a rectangle that is the true distance. For an
//! ellipse it is measured along the ray from the centre, which is close enough for a fade and keeps the value falling
//! steadily towards the edge.

use glam::Vec2;

use crate::geom::Rect;

/// Where a polyline may draw.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Clip {
    /// No clip: every segment draws whole and nothing fades.
    #[default]
    None,
    /// Inside a circle.
    Circle { centre: Vec2, radius: f32 },
    /// Inside an axis-aligned ellipse with these half-axes.
    Ellipse { centre: Vec2, radii: Vec2 },
    /// Inside a rectangle.
    Rect(Rect),
}

impl Clip {
    /// The part of the segment from `a` to `b` inside the region, as `(t0, t1)` with `0 <= t0 < t1 <= 1` along it. `None`
    /// when the segment misses the region or only touches its edge. A start inside the region gives exactly `t0 = 0`,
    /// and an end inside it exactly `t1 = 1`.
    pub fn interval(&self, a: Vec2, b: Vec2) -> Option<(f32, f32)> {
        match *self {
            Clip::None => Some((0.0, 1.0)),
            Clip::Circle { centre, radius } => {
                if !positive(radius) {
                    return None;
                }
                unit_circle_interval((a - centre) / radius, (b - centre) / radius)
            }
            Clip::Ellipse { centre, radii } => {
                if !positive(radii.x) || !positive(radii.y) {
                    return None;
                }
                unit_circle_interval((a - centre) / radii, (b - centre) / radii)
            }
            Clip::Rect(rect) => rect_interval(a, b, rect),
        }
    }

    /// How far `p` is inside the edge, in canvas units: positive inside, negative outside, infinite with no clip.
    pub fn edge_distance(&self, p: Vec2) -> f32 {
        match *self {
            Clip::None => f32::INFINITY,
            Clip::Circle { centre, radius } => radius - (p - centre).length(),
            Clip::Ellipse { centre, radii } => ellipse_edge_distance(p - centre, radii),
            Clip::Rect(rect) => {
                (p.x - rect.min_x()).min(rect.max_x() - p.x).min(p.y - rect.min_y()).min(rect.max_y() - p.y)
            }
        }
    }

    /// Whether `p` is inside the region, edge included.
    pub fn contains(&self, p: Vec2) -> bool {
        self.edge_distance(p) >= 0.0
    }

    /// The opacity at `p` for a fade band `band` units wide inside the edge: 0 at the edge, rising linearly to 1 at
    /// `band` inside it, and 1 everywhere with no band or no clip.
    pub fn fade(&self, p: Vec2, band: f32) -> f32 {
        if !positive(band) {
            return 1.0;
        }
        (self.edge_distance(p) / band).clamp(0.0, 1.0)
    }

    /// The region shrunk by `band` on every side: where a fade band of that width leaves full opacity. `None` when
    /// nothing is left, so every point is in the band. With no clip it stays `Clip::None`.
    pub fn inset(&self, band: f32) -> Option<Clip> {
        match *self {
            Clip::None => Some(Clip::None),
            Clip::Circle { centre, radius } => {
                positive(radius - band).then_some(Clip::Circle { centre, radius: radius - band })
            }
            Clip::Ellipse { centre, radii } => {
                let inner = radii - Vec2::splat(band);
                (positive(inner.x) && positive(inner.y)).then_some(Clip::Ellipse { centre, radii: inner })
            }
            Clip::Rect(rect) => {
                let inner = rect.inset(band, band);
                (positive(inner.w) && positive(inner.h)).then_some(Clip::Rect(inner))
            }
        }
    }
}

/// Whether `v` is greater than zero. False for NaN.
pub(super) fn positive(v: f32) -> bool {
    v > 0.0
}

/// The part of the segment from `a` to `b` inside the unit circle at the origin: the roots of `|a + t (b - a)|^2 = 1`,
/// limited to `0..=1`.
fn unit_circle_interval(a: Vec2, b: Vec2) -> Option<(f32, f32)> {
    let d = b - a;
    let qa = d.length_squared();
    let qb = 2.0 * a.dot(d);
    let qc = a.length_squared() - 1.0;
    if !positive(qa) {
        return (qc < 0.0).then_some((0.0, 1.0));
    }
    let discriminant = qb * qb - 4.0 * qa * qc;
    if !positive(discriminant) {
        return None;
    }
    let root = discriminant.sqrt();
    let t0 = ((-qb - root) / (2.0 * qa)).max(0.0);
    let t1 = ((-qb + root) / (2.0 * qa)).min(1.0);
    (t0 < t1).then_some((t0, t1))
}

/// The part of the segment from `a` to `b` inside `rect` (Liang-Barsky): each edge either cuts the start of the
/// interval (the segment enters across it) or its end (the segment leaves across it).
fn rect_interval(a: Vec2, b: Vec2, rect: Rect) -> Option<(f32, f32)> {
    let d = b - a;
    let edges =
        [(-d.x, a.x - rect.min_x()), (d.x, rect.max_x() - a.x), (-d.y, a.y - rect.min_y()), (d.y, rect.max_y() - a.y)];
    let (mut t0, mut t1) = (0.0_f32, 1.0_f32);
    for (p, q) in edges {
        // Parallel to this edge: inside its half-plane throughout, or never.
        if p.abs() < f32::MIN_POSITIVE {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            t0 = t0.max(r);
            continue;
        }
        t1 = t1.min(r);
    }
    (t0 < t1).then_some((t0, t1))
}

/// How far the offset `v` from an ellipse's centre is inside its edge, measured along the ray from the centre: the
/// ellipse's radius in that direction less the length of `v`.
fn ellipse_edge_distance(v: Vec2, radii: Vec2) -> f32 {
    if !positive(radii.x) || !positive(radii.y) {
        return f32::NEG_INFINITY;
    }
    let scaled = (v / radii).length();
    if scaled < 1e-6 {
        return radii.min_element();
    }
    v.length() * (1.0 / scaled - 1.0)
}

#[cfg(test)]
mod tests {
    use super::Clip;
    use crate::geom::Rect;
    use glam::Vec2;

    const CIRCLE: Clip = Clip::Circle { centre: Vec2::new(10.0, 0.0), radius: 50.0 };

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn a_segment_inside_the_circle_is_kept_whole() {
        assert_eq!(CIRCLE.interval(Vec2::new(0.0, 0.0), Vec2::new(20.0, 10.0)), Some((0.0, 1.0)));
    }

    #[test]
    fn a_segment_outside_the_circle_is_dropped() {
        assert_eq!(CIRCLE.interval(Vec2::new(100.0, 100.0), Vec2::new(120.0, 100.0)), None);
        // The line through the segment crosses the circle, but the segment stops short of it.
        assert_eq!(CIRCLE.interval(Vec2::new(70.0, 0.0), Vec2::new(90.0, 0.0)), None);
    }

    #[test]
    fn a_segment_crossing_the_circle_is_cut_at_the_edge() {
        let (t0, t1) = CIRCLE.interval(Vec2::new(10.0, 0.0), Vec2::new(110.0, 0.0)).expect("it crosses");
        assert_eq!(t0, 0.0, "the start is inside");
        assert!(close(t1, 0.5), "{t1}");
        let (t0, t1) = CIRCLE.interval(Vec2::new(-90.0, 0.0), Vec2::new(110.0, 0.0)).expect("a chord");
        assert!(close(t0, 0.25) && close(t1, 0.75), "{t0} {t1}");
    }

    #[test]
    fn a_tangent_segment_only_touches_and_is_dropped() {
        assert_eq!(CIRCLE.interval(Vec2::new(-40.0, 50.0), Vec2::new(60.0, 50.0)), None);
    }

    #[test]
    fn an_ellipse_cuts_where_the_segment_meets_its_edge() {
        let clip = Clip::Ellipse { centre: Vec2::ZERO, radii: Vec2::new(100.0, 40.0) };
        let (a, b) = (Vec2::new(0.0, -200.0), Vec2::new(0.0, 200.0));
        let (t0, t1) = clip.interval(a, b).expect("it crosses");
        let (enter, exit) = (a.lerp(b, t0), a.lerp(b, t1));
        assert!(close(enter.y, -40.0) && close(exit.y, 40.0), "{enter} {exit}");
        let across = clip.interval(Vec2::new(-200.0, 0.0), Vec2::ZERO).expect("it crosses");
        assert!(close(Vec2::new(-200.0, 0.0).lerp(Vec2::ZERO, across.0).x, -100.0));
    }

    #[test]
    fn a_rectangle_cuts_at_its_edges_and_drops_what_runs_beside_it() {
        let clip = Clip::Rect(Rect::new(0.0, 0.0, 100.0, 40.0));
        let (t0, t1) = clip.interval(Vec2::new(-100.0, 0.0), Vec2::new(100.0, 0.0)).expect("it crosses");
        assert!(close(t0, 0.25) && close(t1, 0.75), "{t0} {t1}");
        assert_eq!(clip.interval(Vec2::new(-100.0, 30.0), Vec2::new(100.0, 30.0)), None, "parallel, above the top");
        assert_eq!(clip.interval(Vec2::new(-10.0, 5.0), Vec2::new(10.0, -5.0)), Some((0.0, 1.0)));
        assert_eq!(clip.interval(Vec2::new(60.0, -100.0), Vec2::new(200.0, 100.0)), None);
    }

    #[test]
    fn no_clip_keeps_everything_at_full_opacity() {
        assert_eq!(Clip::None.interval(Vec2::splat(1e6), Vec2::splat(-1e6)), Some((0.0, 1.0)));
        assert_eq!(Clip::None.fade(Vec2::splat(1e6), 10.0), 1.0);
        assert_eq!(Clip::None.inset(10.0), Some(Clip::None));
    }

    #[test]
    fn a_degenerate_region_keeps_nothing() {
        let flat = Clip::Circle { centre: Vec2::ZERO, radius: 0.0 };
        assert_eq!(flat.interval(Vec2::ZERO, Vec2::X), None);
        let thin = Clip::Ellipse { centre: Vec2::ZERO, radii: Vec2::new(10.0, 0.0) };
        assert_eq!(thin.interval(Vec2::ZERO, Vec2::X), None);
        assert!(!thin.contains(Vec2::ZERO));
    }

    #[test]
    fn the_fade_falls_steadily_to_zero_at_the_edge() {
        for clip in [
            CIRCLE,
            Clip::Ellipse { centre: Vec2::new(10.0, 0.0), radii: Vec2::new(50.0, 30.0) },
            Clip::Rect(Rect::new(10.0, 0.0, 100.0, 100.0)),
        ] {
            let mut last = 2.0;
            for k in 0..=50 {
                let p = Vec2::new(10.0 + k as f32, 0.0);
                let alpha = clip.fade(p, 10.0);
                assert!(alpha <= last, "{clip:?} at {p}: {alpha} after {last}");
                last = alpha;
            }
            assert_eq!(clip.fade(Vec2::new(10.0, 0.0), 10.0), 1.0, "{clip:?}");
            assert_eq!(last, 0.0, "{clip:?}");
        }
        assert_eq!(CIRCLE.fade(Vec2::new(55.0, 0.0), 10.0), 0.5);
        assert_eq!(CIRCLE.fade(Vec2::new(55.0, 0.0), 0.0), 1.0, "no band, no fade");
    }

    #[test]
    fn inset_shrinks_the_region_by_the_band_and_vanishes_when_it_is_too_wide() {
        assert_eq!(CIRCLE.inset(10.0), Some(Clip::Circle { centre: Vec2::new(10.0, 0.0), radius: 40.0 }));
        assert_eq!(CIRCLE.inset(60.0), None);
        let rect = Clip::Rect(Rect::new(0.0, 0.0, 100.0, 40.0));
        assert_eq!(rect.inset(5.0), Some(Clip::Rect(Rect::new(0.0, 0.0, 90.0, 30.0))));
        assert_eq!(rect.inset(20.0), None);
        let ellipse = Clip::Ellipse { centre: Vec2::ZERO, radii: Vec2::new(100.0, 40.0) };
        assert_eq!(ellipse.inset(10.0), Some(Clip::Ellipse { centre: Vec2::ZERO, radii: Vec2::new(90.0, 30.0) }));
    }

    #[test]
    fn contains_includes_the_edge() {
        assert!(CIRCLE.contains(Vec2::new(60.0, 0.0)));
        assert!(!CIRCLE.contains(Vec2::new(60.1, 0.0)));
    }
}
