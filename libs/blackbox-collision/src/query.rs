//! Segment queries against one instance. Spec: `docs/formats/collision.md` ("Query semantics").

use crate::groups::GroupSet;
use crate::math::{
    Vec3, cross, dot, lerp, normalize, scale, segment_point_dist_sq, segment_point_dist_sq_xz, segment_segment_xz,
    segment_triangle_edges, sub,
};
use crate::pack::GROUP_EXCLUSION;
use crate::{Article, Instance, Strip, Triangle};

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
    /// Skip instances that belong to a scenery group (`group != 0`) unless the world has that group enabled.
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
    /// World-space normal, on the side of the segment's start. Ordinary casts normalize it;
    /// `camera_ray_cast` preserves the original barrier's authored inverse-length scaling.
    pub normal: Vec3,
    pub kind: HitKind,
    /// The segment started on the side the surface's own normal points to. For a barrier that is its front:
    /// the side a vehicle is blocked from unless [`crate::BARRIER_TWO_SIDED`] is set in `surface_flags`.
    pub front_facing: bool,
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

/// What the broad phase reads from an instance. A function of the instance alone, so a window keeps it between rays
/// (`crate::face_cache`); kept or not, the values are the same.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Broad {
    centre: Vec3,
    radius: f32,
    upright: bool,
    top: f32,
    bottom: f32,
}

impl Broad {
    pub(crate) fn of(inst: &Instance) -> Self {
        let centre = inst.position();
        Self {
            centre,
            radius: inst.broad_radius(),
            upright: !inst.needs_cross(),
            top: centre[1] + inst.half_height,
            bottom: centre[1] - inst.half_height,
        }
    }

    /// Whether segment `a`-`b` can meet the instance: its circle in xz, and its height band when it is upright.
    fn admits(&self, (a, b): (Vec3, Vec3)) -> bool {
        if segment_point_dist_sq_xz(a, b, self.centre) >= self.radius * self.radius {
            return false;
        }
        let in_band = a[1].min(b[1]) < self.top && a[1].max(b[1]) > self.bottom;
        in_band || !self.upright
    }
}

/// A strip triangle as the face test reads it: the first and last corners, the two edges from the first, and the
/// third vertex's surface and flags. A function of the triangle alone, so a window keeps it between rays.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FaceTriangle {
    p0: Vec3,
    p2: Vec3,
    e1: Vec3,
    e2: Vec3,
    surface: u8,
    flags: u8,
}

impl FaceTriangle {
    pub(crate) fn of(tri: &Triangle) -> Self {
        let [p0, p1, p2] = tri.pts;
        Self { p0, p2, e1: sub(p1, p0), e2: sub(p2, p0), surface: tri.surface, flags: tri.flags }
    }

    /// The local face normal, `(p1 - p0) x (p0 - p2)` normalized.
    fn normal(&self) -> Vec3 {
        normalize(cross(self.e1, sub(self.p0, self.p2))).unwrap_or([0.0, 1.0, 0.0])
    }
}

/// Where the face test takes a strip's triangles from: decoded on the spot ([`Decode`]) or kept by a window.
pub(crate) trait StripFaces {
    /// Calls `each` with the triangles of strip `index` of the article (`strip`), in strip order.
    fn each_triangle(&mut self, index: usize, strip: &Strip, each: impl FnMut(&FaceTriangle));
}

/// Decodes each strip's triangles as they are tested: the plain cast.
pub(crate) struct Decode;

impl StripFaces for Decode {
    fn each_triangle(&mut self, _: usize, strip: &Strip, mut each: impl FnMut(&FaceTriangle)) {
        for tri in strip.triangles() {
            each(&FaceTriangle::of(&tri));
        }
    }
}

/// The instance a cast tests: its placement and geometry, the world's scenery groups, and the identity hits carry.
pub(crate) struct Subject<'a> {
    pub inst: &'a Instance,
    pub article: &'a Article,
    pub groups: &'a GroupSet,
    pub identity: (u32, usize),
}

/// The nearest hit of segment `a`-`b` (world space, already prepared) on one instance.
pub(crate) fn cast_instance(
    inst: &Instance,
    article: &Article,
    seg: (Vec3, Vec3),
    opts: &RayOptions,
    groups: &GroupSet,
    identity: (u32, usize),
    accept: &mut impl FnMut(&Hit) -> bool,
) -> Option<Hit> {
    let subject = Subject { inst, article, groups, identity };
    cast_subject(&subject, Broad::of, &mut Decode, seg, opts, accept)
}

/// [`cast_instance`] with the broad-phase values from `broad` and the triangles from `faces`. Every path runs this one
/// function, so kept values are tested with the very operations, in the very order, of the plain cast.
pub(crate) fn cast_subject(
    subject: &Subject<'_>,
    broad: impl FnOnce(&Instance) -> Broad,
    faces: &mut impl StripFaces,
    (a, b): (Vec3, Vec3),
    opts: &RayOptions,
    accept: &mut impl FnMut(&Hit) -> bool,
) -> Option<Hit> {
    let Subject { inst, article, groups, identity } = *subject;
    // An enabled group is in the world: its baked group-exclusion bits (on the instance and its barriers) do not apply.
    let group_on = inst.group != 0 && groups.contains(inst.group);
    if opts.skip_groups && inst.group != 0 && !group_on {
        return None;
    }
    let exclude = if group_on { opts.exclude & !u32::from(GROUP_EXCLUSION) } else { opts.exclude };
    if u32::from(inst.flags) & exclude != 0 {
        return None;
    }
    if !broad(inst).admits((a, b)) {
        return None;
    }

    let (la, lb) = (inst.to_local(a), inst.to_local(b));
    let mut best: Option<Hit> = None;
    let mut consider = |c: Candidate| {
        if best.as_ref().is_some_and(|h| c.t >= h.t) {
            return;
        }
        let point = inst.to_world(lerp(la, lb, c.t));
        let mut normal = inst.dir_to_world(c.normal);
        let front_facing = dot(normal, sub(a, point)) >= 0.0;
        if !front_facing {
            normal = scale(normal, -1.0);
        }
        let hit = Hit {
            t: c.t,
            point,
            normal,
            kind: c.kind,
            front_facing,
            surface_hash: article.surface_hash(c.surface).unwrap_or(0),
            surface_index: c.surface,
            surface_flags: c.flags,
            section: identity.0,
            instance: identity.1,
        };
        if accept(&hit) {
            best = Some(hit);
        }
    };

    if opts.faces {
        for (index, strip) in article.strips.iter().enumerate() {
            if u32::from(strip.flags) & opts.exclude != 0
                || segment_point_dist_sq(la, lb, strip.center) >= strip.radius * strip.radius
            {
                continue;
            }
            faces.each_triangle(index, strip, |tri| {
                if u32::from(tri.flags) & opts.exclude != 0 {
                    return;
                }
                let Some(t) = segment_triangle_edges(la, lb, tri.p0, tri.e1, tri.e2) else { return };
                consider(Candidate {
                    t,
                    kind: HitKind::Face,
                    normal: tri.normal(),
                    surface: tri.surface,
                    flags: tri.flags,
                });
            });
        }
    }
    if opts.barriers {
        for bar in &article.barriers {
            if u32::from(bar.flags) & exclude != 0 {
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

    best
}
