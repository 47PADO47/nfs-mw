//! A set of loaded collision packs plus the grid that finds them: the engine's `WCollisionAssets`
//! and `WCollisionMgr` in one value.

use std::collections::BTreeMap;

use crate::grid::Grid;
use crate::math::{Vec3, dot, sub};
use crate::query::{cast_instance, prepare_segment};
use crate::{CollisionPack, Hit, HitKind, RayOptions};

#[derive(Debug, Default)]
pub struct CollisionWorld {
    grid: Option<Grid>,
    packs: BTreeMap<u32, CollisionPack>,
}

impl CollisionWorld {
    /// A world that picks candidates with `grid`, or tests every loaded instance without one.
    pub fn new(grid: Option<Grid>) -> Self {
        Self { grid, packs: BTreeMap::new() }
    }

    /// Adds (or replaces) a section's pack, applying the game's post-load exclusion flags.
    pub fn insert(&mut self, mut pack: CollisionPack) {
        pack.apply_group_exclusion();
        self.packs.insert(pack.section, pack);
    }

    pub fn remove(&mut self, section: u32) -> Option<CollisionPack> {
        self.packs.remove(&section)
    }

    pub fn pack(&self, section: u32) -> Option<&CollisionPack> {
        self.packs.get(&section)
    }

    pub fn grid(&self) -> Option<&Grid> {
        self.grid.as_ref()
    }

    pub fn pack_count(&self) -> usize {
        self.packs.len()
    }

    /// The nearest hit along the segment from `from` to `to` (physics space: x right, y up,
    /// z forward), or `None`. Instances whose pack is not loaded are ignored.
    pub fn ray_cast(&self, from: Vec3, to: Vec3, opts: &RayOptions) -> Option<Hit> {
        self.ray_cast_filtered(from, to, opts, |_| true)
    }

    /// The nearest hit accepted by `accept`. The predicate is applied before choosing a winner,
    /// including within one article, so a rejected surface never hides a farther accepted one.
    /// It receives world-space data and the candidate's actual section and instance identity.
    /// Farther candidates may be culled without calling the predicate; do not depend on its call count.
    pub fn ray_cast_filtered(
        &self,
        from: Vec3,
        to: Vec3,
        opts: &RayOptions,
        mut accept: impl FnMut(&Hit) -> bool,
    ) -> Option<Hit> {
        let seg = prepare_segment(from, to)?;
        let mut best: Option<Hit> = None;
        let mut test = |pack: &CollisionPack, index: usize| {
            let (Some(inst), Some(article)) = (pack.instances.get(index), pack.article_of(index)) else { return };
            if let Some(hit) = cast_instance(inst, article, seg, opts, (pack.section, index), &mut accept)
                && best.as_ref().is_none_or(|b| closer(&hit, b, seg.0))
            {
                best = Some(hit);
            }
        };
        match &self.grid {
            Some(grid) => {
                for r in grid.instances_along(seg.0, seg.1) {
                    if let Some(pack) = self.packs.get(&r.section()) {
                        test(pack, r.index());
                    }
                }
            }
            None => {
                for pack in self.packs.values() {
                    for index in 0..pack.instances.len() {
                        test(pack, index);
                    }
                }
            }
        }
        best
    }

    /// The first surface met going down from `(x, top, z)` to `(x, bottom, z)`: ground height
    /// queries. Faces only.
    pub fn ground(&self, x: f32, z: f32, top: f32, bottom: f32) -> Option<Hit> {
        let opts = RayOptions { barriers: false, ..RayOptions::default() };
        self.ray_cast([x, top, z], [x, bottom, z], &opts)
    }
}

/// Whether `a` is nearer the segment start than `b`; at equal distance a face beats a barrier.
fn closer(a: &Hit, b: &Hit, start: Vec3) -> bool {
    let d = |h: &Hit| {
        let v = sub(h.point, start);
        dot(v, v)
    };
    d(a) < d(b) || (d(a) == d(b) && a.kind == HitKind::Face && b.kind == HitKind::Barrier)
}
