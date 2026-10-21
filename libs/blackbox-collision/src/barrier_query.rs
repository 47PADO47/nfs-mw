//! Nearby authored wall geometry. Spec: docs/specs/world-barrier-list.md.

use crate::math::{Vec3, sub};
use crate::{Barrier, CollisionPack, CollisionWorld, GROUP_EXCLUSION, RayOptions};

#[cfg(test)]
mod tests;

/// Copied world barrier; streaming removal cannot invalidate its geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldBarrier {
    /// World endpoints, original surface index/flags and authored inverse xz length.
    pub barrier: Barrier,
    /// Authored world-horizontal normal, without normalization or facing reversal.
    pub normal: Vec3,
    /// Instance-local horizontal point-to-segment distance used for list ordering.
    pub distance_sq: f32,
    pub surface_hash: u32,
    pub section: u32,
    pub instance: usize,
}

impl CollisionWorld {
    /// Collect nearby barriers, ordered by original local squared segment distance.
    /// `options.barriers` and `exclude` apply; scenery groups always require enabled
    /// membership, independently of ray-only `faces` and `skip_groups` options.
    /// The caller supplies its body search radius; invalid input clears `out`.
    pub fn barriers_near_into(&self, center: Vec3, radius: f32, options: &RayOptions, out: &mut Vec<WorldBarrier>) {
        out.clear();
        if !options.barriers || !radius.is_finite() || radius < 0.0 || !center.into_iter().all(f32::is_finite) {
            return;
        }
        let mut test = |pack: &CollisionPack, index| collect(self, pack, index, center, radius, options.exclude, out);
        match self.grid() {
            Some(grid) => {
                let mut refs = self.take_candidates();
                refs.clear();
                let (r0, c0) = grid.cell_of(center[0] - radius, center[2] - radius);
                let (r1, c1) = grid.cell_of(center[0] + radius, center[2] + radius);
                for col in c0..=c1 {
                    for row in r0..=r1 {
                        let Some(node) = grid.node(grid.node_index(row, col)) else { continue };
                        for reference in &node.instances {
                            if refs.contains(reference) {
                                continue;
                            }
                            refs.push(*reference);
                            if let Some(pack) = self.pack(reference.section()) {
                                test(pack, reference.index());
                            }
                        }
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
        for pack in self.ungridded_packs() {
            for index in 0..pack.instances.len() {
                test(pack, index);
            }
        }
        out.sort_by(|a, b| a.distance_sq.total_cmp(&b.distance_sq));
    }
}

fn collect(
    world: &CollisionWorld,
    pack: &CollisionPack,
    index: usize,
    center: Vec3,
    radius: f32,
    exclude: u32,
    out: &mut Vec<WorldBarrier>,
) {
    let (Some(instance), Some(article)) = (pack.instances.get(index), pack.article_of(index)) else { return };
    if instance.group != 0 && !world.group_enabled(instance.group) {
        return;
    }
    let exclude = match instance.group {
        0 => exclude,
        _ => exclude & !u32::from(GROUP_EXCLUSION),
    };
    if u32::from(instance.flags) & exclude != 0 {
        return;
    }
    let offset = sub(instance.position(), center);
    if offset[0] * offset[0] + offset[2] * offset[2] >= (radius + instance.broad_radius()).powi(2) {
        return;
    }
    let point = instance.to_local(center);
    for local in &article.barriers {
        if u32::from(local.flags) & exclude != 0 {
            continue;
        }
        let distance_sq = authored_distance_sq(*local, point);
        if distance_sq >= radius * radius || point[1] + radius <= local.y_bottom() || point[1] - radius > local.y_top()
        {
            continue;
        }
        let barrier = Barrier { p0: instance.to_world(local.p0), p1: instance.to_world(local.p1), ..*local };
        let delta = sub(barrier.p1, barrier.p0);
        out.push(WorldBarrier {
            barrier,
            normal: [delta[2] * barrier.inv_xz_len, 0.0, -delta[0] * barrier.inv_xz_len],
            distance_sq,
            surface_hash: article.surface_hash(barrier.surface).unwrap_or(0),
            section: pack.section,
            instance: index,
        });
    }
}

fn authored_distance_sq(barrier: Barrier, point: Vec3) -> f32 {
    let delta = sub(barrier.p1, barrier.p0);
    let offset = sub(point, barrier.p0);
    let fraction =
        ((offset[0] * delta[0] + offset[2] * delta[2]) * (barrier.inv_xz_len * barrier.inv_xz_len)).clamp(0.0, 1.0);
    let x = delta[0] * fraction - offset[0];
    let z = delta[2] * fraction - offset[2];
    x * x + z * z
}
