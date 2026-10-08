//! Segment queries against one instance. Spec: `docs/formats/collision.md` ("Query semantics").

use crate::math::{
    Vec3, cross, dot, lerp, normalize, scale, segment_point_dist_sq, segment_point_dist_sq_xz, segment_segment_xz,
    segment_triangle, sub,
};
use crate::{Article, Instance};

/// Segments shorter than this are lengthened to it, as the game does.
const MIN_SEGMENT: f32 = 0.01;

/// What a ray cast tests and skips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RayOptions {
    /// Test triangle strips (the ground and solid surfaces).
    pub faces: bool,
    /// Test barriers (walls).
    pub barriers: bool,
    /// Skip anything whose flags share a bit with this mask: instances (runtime flags), strips
    /// (strip flags), triangles and barriers (surface flags). 0 skips nothing.
    pub exclude: u32,
    /// Skip instances that belong to a scenery group (`group != 0`).
    pub skip_groups: bool,
}

impl Default for RayOptions {
    fn default() -> Self {
        Self { faces: true, barriers: true, exclude: 0, skip_groups: false }
    }
}

/// What was hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    Face,
    Barrier,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// Fraction of the way from the start to the end of the (possibly lengthened) segment.
    pub t: f32,
    /// World-space hit point.
    pub point: Vec3,
    /// World-space unit normal, on the side of the segment's start.
    pub normal: Vec3,
    pub kind: HitKind,
    /// Surface type hash from the article's table (`simsurface` key), 0 when unknown.
    pub surface_hash: u32,
    pub surface_index: u8,
    pub surface_flags: u8,
    /// Section of the pack that was hit (0 when the query ran on a bare pack).
    pub section: u32,
    /// Index of the instance in its pack.
    pub instance: usize,
}

/// The ends of a segment, lengthened to [`MIN_SEGMENT`] when shorter. `None` for a point.
pub(crate) fn prepare_segment(a: Vec3, b: Vec3) -> Option<(Vec3, Vec3)> {
    let d = sub(b, a);
    let len = dot(d, d).sqrt();
    if len == 0.0 {
        None
    } else if len < MIN_SEGMENT {
        Some((a, lerp(a, b, MIN_SEGMENT / len)))
    } else {
        Some((a, b))
    }
}

struct Candidate {
    t: f32,
    kind: HitKind,
    normal: Vec3,
    surface: u8,
    flags: u8,
}

/// The nearest hit of segment `a`-`b` (world space, already prepared) on one instance.
pub(crate) fn cast_instance(
    inst: &Instance,
    article: &Article,
    (a, b): (Vec3, Vec3),
    opts: &RayOptions,
) -> Option<Hit> {
    if u32::from(inst.flags) & opts.exclude != 0 || (opts.skip_groups && inst.group != 0) {
        return None;
    }
    // Broad phase: the instance circle in xz, and its height band when it is upright.
    let centre = inst.position();
    let r = inst.broad_radius();
    if segment_point_dist_sq_xz(a, b, centre) >= r * r {
        return None;
    }
    let in_band = a[1].min(b[1]) < centre[1] + inst.half_height && a[1].max(b[1]) > centre[1] - inst.half_height;
    if !inst.needs_cross() && !in_band {
        return None;
    }

    let (la, lb) = (inst.to_local(a), inst.to_local(b));
    let mut best: Option<Candidate> = None;
    let mut consider = |c: Candidate| {
        if best.as_ref().is_none_or(|h| c.t < h.t) {
            best = Some(c);
        }
    };

    if opts.faces {
        for strip in &article.strips {
            if u32::from(strip.flags) & opts.exclude != 0
                || segment_point_dist_sq(la, lb, strip.center) >= strip.radius * strip.radius
            {
                continue;
            }
            for tri in strip.triangles() {
                if u32::from(tri.flags) & opts.exclude != 0 {
                    continue;
                }
                if let Some(t) = segment_triangle(la, lb, &tri.pts) {
                    let n = cross(sub(tri.pts[1], tri.pts[0]), sub(tri.pts[0], tri.pts[2]));
                    consider(Candidate {
                        t,
                        kind: HitKind::Face,
                        normal: normalize(n).unwrap_or([0.0, 1.0, 0.0]),
                        surface: tri.surface,
                        flags: tri.flags,
                    });
                }
            }
        }
    }
    if opts.barriers {
        for bar in &article.barriers {
            if u32::from(bar.flags) & opts.exclude != 0 {
                continue;
            }
            if let Some(t) = segment_segment_xz(la, lb, bar.p0, bar.p1) {
                let y = lerp(la, lb, t)[1];
                if y > bar.y_bottom() && y < bar.y_top() {
                    consider(Candidate {
                        t,
                        kind: HitKind::Barrier,
                        normal: bar.normal(),
                        surface: bar.surface,
                        flags: bar.flags,
                    });
                }
            }
        }
    }

    let c = best?;
    let point = inst.to_world(lerp(la, lb, c.t));
    // Orient the normal towards the start of the segment.
    let mut normal = inst.dir_to_world(c.normal);
    if dot(normal, sub(a, point)) < 0.0 {
        normal = scale(normal, -1.0);
    }
    Some(Hit {
        t: c.t,
        point,
        normal,
        kind: c.kind,
        surface_hash: article.surface_hash(c.surface).unwrap_or(0),
        surface_index: c.surface,
        surface_flags: c.flags,
        section: 0,
        instance: 0,
    })
}
