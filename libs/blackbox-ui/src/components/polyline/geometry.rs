//! The pure geometry of a stroked polyline: the quad of a segment, the fill of a join, the pieces a segment is cut into
//! across a fade band, and the walk over the distinct points.
//!
//! The presenter draws an image node as the unit quad (-0.5 to 0.5 on each axis) through the node's `world` matrix, in one
//! colour. A matrix whose x and y columns are any two vectors draws the parallelogram they span, so a rotated rectangle is
//! a matrix with the segment's direction (times its length) in x and the normal (times the width) in y. A parallelogram is
//! all one node can be: never a triangle, never a general quadrilateral. The joins are built within that limit.

use glam::{Mat4, Vec2, Vec4};

use super::LineJoin;
use super::clip::positive;

/// Two points closer than this are the same point, and the segment between them is skipped.
pub(super) const MIN_SEGMENT: f32 = 1e-3;

/// A join whose outer notch is narrower than this, in canvas units, gets no fill: nothing would show.
const MIN_NOTCH: f32 = 0.01;

/// A turn sharper than this (the cosine of half the turn) is a reversal: the segments fold back over each other and
/// leave no notch to fill.
const MIN_HALF_COS: f32 = 1e-3;

/// The most pieces one end of a segment is cut into across the fade band, however long it is.
pub(super) const MAX_BAND_PIECES: usize = 16;

/// The world transform of the parallelogram centred on `centre` whose unit-quad x side becomes `along` and y side
/// `across`, at depth `z`. With perpendicular axes it is a rectangle turned to the direction of `along`: the same as a
/// translation, a rotation about z and a scale by the two lengths.
pub(super) fn quad(centre: Vec2, along: Vec2, across: Vec2, z: f32) -> Mat4 {
    Mat4::from_cols(
        Vec4::new(along.x, along.y, 0.0, 0.0),
        Vec4::new(across.x, across.y, 0.0, 0.0),
        Vec4::Z,
        Vec4::new(centre.x, centre.y, z, 1.0),
    )
}

/// What fills the corner where one segment turns into the next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Joint {
    /// Nothing: the segments run on straight, turn too little to leave a gap, or fold back over each other.
    None,
    /// A mitre: both segments run on this far past the joint, so their outer edges meet in the mitre's point. No extra
    /// node. Inside a mitre limit of 2 the extension also stays within the stroke on the inner side.
    Extend(f32),
    /// A bevel: one rectangle over the outer notch, given as its centre's offset from the joint and its two axes. Its
    /// outer edge is the bevel line between the two outer corners; the rest of it lies over the two segments.
    Bevel { offset: Vec2, along: Vec2, across: Vec2 },
}

/// The fill of the joint between a segment running in the unit direction `t_in` and the next running in `t_out`, for a
/// stroke `half` units either side of the centre line. A mitre longer than its limit (the mitre length over the half
/// width) falls back to a bevel.
pub(super) fn joint(t_in: Vec2, t_out: Vec2, half: f32, join: LineJoin) -> Joint {
    let cos_turn = t_in.dot(t_out).clamp(-1.0, 1.0);
    // The cosine and sine of half the turn.
    let half_cos = ((1.0 + cos_turn) / 2.0).sqrt();
    let half_sin = ((1.0 - cos_turn) / 2.0).sqrt();
    if half * half_sin < MIN_NOTCH || half_cos < MIN_HALF_COS {
        return Joint::None;
    }
    if let LineJoin::Mitre { limit } = join
        && 1.0 / half_cos <= limit
    {
        return Joint::Extend(half * half_sin / half_cos);
    }
    // The outer side is the one the path turns away from: the right of a left turn, the left of a right turn.
    let side = -t_in.perp_dot(t_out).signum();
    let normal = ((t_in.perp() + t_out.perp()) * side).normalize_or_zero();
    Joint::Bevel {
        offset: normal * (half * half_cos / 2.0),
        along: normal.perp() * (2.0 * half * half_sin),
        across: normal * (half * half_cos),
    }
}

/// One piece of a segment's drawn part: its interval along the segment, and whether it starts or ends that part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Piece {
    pub t0: f32,
    pub t1: f32,
    pub first: bool,
    pub last: bool,
}

/// The pieces of the interval `span` of a segment. `full` is the part of `span` at full opacity (inside the fade band),
/// or `None` when all of it is in the band. `step` is the longest band piece, as a fraction of the segment; `None` means
/// no fade, so the span is one piece. The full part is always one piece: only the band needs the steps.
pub(super) fn pieces(span: (f32, f32), full: Option<(f32, f32)>, step: Option<f32>) -> Pieces {
    let (lo, hi) = span;
    let Some(step) = step else { return Pieces::new([(lo, hi, 1), (hi, hi, 0), (hi, hi, 0)]) };
    let count = |from: f32, to: f32| {
        if !positive(to - from) {
            return 0;
        }
        ((to - from) / step.max(f32::MIN_POSITIVE)).ceil().clamp(1.0, MAX_BAND_PIECES as f32) as usize
    };
    let Some((f0, f1)) = full.map(|(f0, f1)| (f0.max(lo), f1.min(hi))).filter(|(f0, f1)| f0 < f1) else {
        return Pieces::new([(lo, hi, count(lo, hi)), (hi, hi, 0), (hi, hi, 0)]);
    };
    Pieces::new([(lo, f0, count(lo, f0)), (f0, f1, 1), (f1, hi, count(f1, hi))])
}

/// The iterator [`pieces`] returns: up to three ranges, each cut into equal pieces, walked in order. It allocates nothing.
pub(super) struct Pieces {
    ranges: [(f32, f32, usize); 3],
    range: usize,
    k: usize,
    emitted: usize,
    total: usize,
}

impl Pieces {
    fn new(ranges: [(f32, f32, usize); 3]) -> Self {
        let total = ranges.iter().map(|r| r.2).sum();
        Self { ranges, range: 0, k: 0, emitted: 0, total }
    }
}

impl Iterator for Pieces {
    type Item = Piece;

    fn next(&mut self) -> Option<Piece> {
        while let Some(&(lo, hi, n)) = self.ranges.get(self.range) {
            if self.k >= n {
                self.range += 1;
                self.k = 0;
                continue;
            }
            let k = self.k;
            self.k += 1;
            self.emitted += 1;
            return Some(Piece {
                t0: cut(lo, hi, k, n),
                t1: cut(lo, hi, k + 1, n),
                first: self.emitted == 1,
                last: self.emitted == self.total,
            });
        }
        None
    }
}

/// The `k`th of `n` equal cuts from `lo` to `hi`. The last cut is exactly `hi`, so neighbouring ranges meet without a gap.
fn cut(lo: f32, hi: f32, k: usize, n: usize) -> f32 {
    if k >= n {
        return hi;
    }
    lo + (hi - lo) * k as f32 / n as f32
}

/// The points of `points` that start a new segment: finite, and at least [`MIN_SEGMENT`] from the point before, so no
/// segment has zero length. It allocates nothing.
pub(super) fn distinct(points: &[Vec2]) -> impl Iterator<Item = Vec2> + '_ {
    let mut last: Option<Vec2> = None;
    points.iter().copied().filter(move |&p| {
        if !p.is_finite() {
            return false;
        }
        if let Some(before) = last
            && before.distance_squared(p) < MIN_SEGMENT * MIN_SEGMENT
        {
            return false;
        }
        last = Some(p);
        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn close(a: Vec2, b: Vec2) -> bool {
        a.distance(b) < 1e-4
    }

    #[test]
    fn a_quad_spans_its_axes_about_its_centre() {
        let world = quad(Vec2::new(10.0, 5.0), Vec2::new(0.0, 8.0), Vec2::new(-2.0, 0.0), 3.0);
        let corner = |x: f32, y: f32| world.transform_point3(Vec3::new(x, y, 0.0));
        assert_eq!(corner(-0.5, -0.5), Vec3::new(11.0, 1.0, 3.0));
        assert_eq!(corner(0.5, 0.5), Vec3::new(9.0, 9.0, 3.0));
        // The same as turning a 8 x 2 rectangle a quarter turn.
        let turned = Mat4::from_translation(Vec3::new(10.0, 5.0, 3.0))
            * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2)
            * Mat4::from_scale(Vec3::new(8.0, 2.0, 1.0));
        assert!(world.abs_diff_eq(turned, 1e-5));
    }

    #[test]
    fn straight_on_and_reversed_joints_need_no_fill() {
        assert_eq!(joint(Vec2::X, Vec2::X, 2.0, LineJoin::Bevel), Joint::None);
        assert_eq!(joint(Vec2::X, -Vec2::X, 2.0, LineJoin::Bevel), Joint::None);
        assert_eq!(joint(Vec2::X, Vec2::new(1.0, 0.001).normalize(), 2.0, LineJoin::Bevel), Joint::None, "too slight");
    }

    #[test]
    fn a_right_angle_mitre_runs_each_segment_on_by_the_half_width() {
        let Joint::Extend(by) = joint(Vec2::X, Vec2::Y, 3.0, LineJoin::Mitre { limit: 2.0 }) else { panic!("a mitre") };
        assert!((by - 3.0).abs() < 1e-5, "{by}");
    }

    #[test]
    fn a_mitre_past_its_limit_falls_back_to_a_bevel() {
        let sharp = Vec2::new(-1.0, 0.2).normalize();
        assert!(matches!(joint(Vec2::X, sharp, 3.0, LineJoin::Mitre { limit: 2.0 }), Joint::Bevel { .. }));
        assert!(matches!(joint(Vec2::X, sharp, 3.0, LineJoin::Mitre { limit: 20.0 }), Joint::Extend(_)));
    }

    #[test]
    fn a_bevel_covers_the_outer_notch_with_its_outer_edge_on_the_bevel_line() {
        // Heading +x, then turning to +y. The outer side is -y for the first segment and +x for the second.
        let half = 2.0;
        let Joint::Bevel { offset, along, across } = joint(Vec2::X, Vec2::Y, half, LineJoin::Bevel) else {
            panic!("a bevel")
        };
        let world = quad(offset, along, across, 0.0);
        let corner = |x: f32, y: f32| world.transform_point3(Vec3::new(x, y, 0.0)).truncate();
        let corners = [corner(-0.5, -0.5), corner(0.5, -0.5), corner(0.5, 0.5), corner(-0.5, 0.5)];
        // The two outer corners of the segments' ends are corners of the fill.
        for outer in [Vec2::new(0.0, -half), Vec2::new(half, 0.0)] {
            assert!(corners.iter().any(|&c| close(c, outer)), "{outer} not in {corners:?}");
        }
        // Its inner edge has the joint at its middle, and its outer edge is the bevel line, half * cos(45 degrees) out.
        assert!(close(corner(0.0, -0.5), Vec2::ZERO), "{}", corner(0.0, -0.5));
        assert!(close(corner(0.0, 0.5), Vec2::new(1.0, -1.0)), "{}", corner(0.0, 0.5));
        assert!((along.length() - 2.0 * half * std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-4);
    }

    #[test]
    fn a_left_and_a_right_turn_fill_opposite_sides() {
        let Joint::Bevel { offset: left, .. } = joint(Vec2::X, Vec2::Y, 2.0, LineJoin::Bevel) else { panic!() };
        let Joint::Bevel { offset: right, .. } = joint(Vec2::X, -Vec2::Y, 2.0, LineJoin::Bevel) else { panic!() };
        assert!(left.y < 0.0 && right.y > 0.0, "{left} {right}");
    }

    #[test]
    fn without_a_fade_the_span_is_one_piece() {
        let all: Vec<Piece> = pieces((0.2, 0.9), None, None).collect();
        assert_eq!(all, vec![Piece { t0: 0.2, t1: 0.9, first: true, last: true }]);
    }

    #[test]
    fn the_band_is_cut_into_steps_and_the_full_part_is_one_piece() {
        let all: Vec<Piece> = pieces((0.0, 1.0), Some((0.3, 0.8)), Some(0.1)).collect();
        // 0.0..0.3 in three steps, 0.3..0.8 whole, 0.8..1.0 in two steps.
        assert_eq!(all.len(), 6);
        assert_eq!(all[3].t0, 0.3);
        assert_eq!(all[3].t1, 0.8);
        for pair in all.windows(2) {
            assert_eq!(pair[0].t1, pair[1].t0, "the pieces meet without a gap");
        }
        assert_eq!((all[0].t0, all[5].t1), (0.0, 1.0));
        assert!(all[0].first && !all[0].last && all[5].last && !all[5].first);
        assert_eq!(all.iter().filter(|p| p.first || p.last).count(), 2);
    }

    #[test]
    fn a_span_all_in_the_band_is_cut_whole_and_the_cuts_are_capped() {
        assert_eq!(pieces((0.0, 0.5), None, Some(0.1)).count(), 5);
        assert_eq!(pieces((0.0, 1.0), None, Some(1e-6)).count(), MAX_BAND_PIECES);
        // A full part outside the span leaves the span in the band.
        assert_eq!(pieces((0.0, 0.5), Some((0.6, 0.9)), Some(0.1)).count(), 5);
    }

    #[test]
    fn distinct_skips_repeats_and_non_finite_points() {
        let points = [
            Vec2::ZERO,
            Vec2::ZERO,
            Vec2::new(1e-5, 0.0),
            Vec2::new(f32::NAN, 1.0),
            Vec2::X,
            Vec2::new(f32::INFINITY, 0.0),
            Vec2::X,
            Vec2::Y,
        ];
        let kept: Vec<Vec2> = distinct(&points).collect();
        assert_eq!(kept, vec![Vec2::ZERO, Vec2::X, Vec2::Y]);
        assert_eq!(distinct(&[]).count(), 0);
    }
}
