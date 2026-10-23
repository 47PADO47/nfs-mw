//! Original camera-only barrier query. Spec: docs/specs/camera-barrier-query.md.

use crate::math::{Vec3, dot, scale, segment_point_dist_sq_xz, sub};
use crate::query::prepare_segment;
use crate::{Article, CollisionWorld, Hit, HitKind, Instance};

#[cfg(test)]
mod tests;

impl CollisionWorld {
    /// Original camera barriers, including its raw endpoint tolerance and authored normal scale.
    /// Nonzero scenery groups are tested only when the caller reports them active.
    pub fn camera_ray_cast(&self, from: Vec3, to: Vec3, mut group_enabled: impl FnMut(u16) -> bool) -> Option<Hit> {
        let segment = prepare_segment(from, to)?;
        let mut best: Option<(f32, Hit)> = None;
        let mut test = |pack: &crate::CollisionPack, index: usize| {
            let (Some(inst), Some(article)) = (pack.instances.get(index), pack.article_of(index)) else { return };
            if inst.group != 0 && !group_enabled(inst.group) {
                return;
            }
            if let Some(hit) = cast_instance(inst, article, segment, (pack.section, index))
                && best.as_ref().is_none_or(|old| hit.0 < old.0)
            {
                best = Some(hit);
            }
        };
        match self.grid() {
            Some(grid) => {
                let mut refs = self.take_candidates();
                grid.instances_along_into(segment.0, segment.1, &mut refs);
                for reference in &refs {
                    if let Some(pack) = self.pack(reference.section()) {
                        test(pack, reference.index());
                    }
                }
                self.return_candidates(refs);
            }
            None => {
                for pack in self.packs() {
                    for index in 0..pack.instances.len() {
                        test(pack, index);
                    }
                }
            }
        }
        best.map(|(_, hit)| hit)
    }
}

fn distance_squared(a: Vec3, b: Vec3) -> f32 {
    let d: [f64; 3] = std::array::from_fn(|i| f64::from(a[i]) - f64::from(b[i]));
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]) as f32
}

fn cast_instance(
    inst: &Instance,
    article: &Article,
    (a, b): (Vec3, Vec3),
    identity: (u32, usize),
) -> Option<(f32, Hit)> {
    let center = inst.position();
    if segment_point_dist_sq_xz(a, b, center) >= inst.radius * inst.radius {
        return None;
    }
    if !inst.needs_cross()
        && !(a[1].min(b[1]) < center[1] + inst.half_height && a[1].max(b[1]) > center[1] - inst.half_height)
    {
        return None;
    }
    let (la, lb) = (inst.to_local(a), inst.to_local(b));
    let mut best: Option<(f32, Hit)> = None;
    for barrier in &article.barriers {
        let Some(t) = crossing(la, lb, barrier.p0, barrier.p1) else { continue };
        let point = std::array::from_fn(|i| {
            let delta = if i == 1 { f64::from(lb[i]) - f64::from(la[i]) } else { f64::from(lb[i] - la[i]) };
            (f64::from(la[i]) + delta * t) as f32
        });
        if point[1] <= barrier.y_bottom() || point[1] >= barrier.y_top() {
            continue;
        }
        let distance = distance_squared(point, la);
        if best.as_ref().is_some_and(|(old, _)| distance >= *old) {
            continue;
        }
        let point = inst.to_world(point);
        let p = inst.to_world(barrier.p0);
        let q = inst.to_world(barrier.p1);
        let mut normal = [(q[2] - p[2]) * barrier.inv_xz_len, 0.0, -(q[0] - p[0]) * barrier.inv_xz_len];
        let front_facing = dot(normal, sub(a, point)) >= 0.0;
        if !front_facing {
            normal = scale(normal, -1.0);
        }
        best = Some((
            distance,
            Hit {
                t: t as f32,
                point,
                normal,
                kind: HitKind::Barrier,
                front_facing,
                surface_hash: article.surface_hash(barrier.surface).unwrap_or(0),
                surface_index: barrier.surface,
                surface_flags: barrier.flags,
                section: identity.0,
                instance: identity.1,
                group: inst.group,
            },
        ));
    }
    best
}

fn crossing(a: Vec3, b: Vec3, p: Vec3, q: Vec3) -> Option<f64> {
    const EPS: f64 = 0.0001_f32 as f64;
    let dx1 = b[0] - a[0];
    let dz1 = b[2] - a[2];
    let dx2 = q[0] - p[0];
    let dz2 = q[2] - p[2];
    let denominator = (f64::from(dz2) * f64::from(dx1) - f64::from(dx2) * f64::from(dz1)) as f32;
    if denominator == 0.0 {
        return None;
    }
    let ex = a[0] - p[0];
    let ez = a[2] - p[2];
    let ray_wide = f64::from(ez) * f64::from(dx2) - f64::from(ex) * f64::from(dz2);
    let ray = ray_wide as f32;
    if !((ray_wide >= -EPS && ray <= denominator) || (f64::from(ray) <= EPS && ray >= denominator)) {
        return None;
    }
    let wall = f64::from(ez) * f64::from(dx1) - f64::from(ex) * f64::from(dz1);
    let den = f64::from(denominator);
    ((wall >= -EPS && wall <= den) || (wall <= EPS && wall >= den)).then(|| f64::from(ray) / den)
}
