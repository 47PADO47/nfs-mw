//! A set of loaded collision packs plus the grid that finds them: the engine's `WCollisionAssets`
//! and `WCollisionMgr` in one value.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::grid::{Grid, InstanceRef};
use crate::math::{Vec3, dot, sub};
use crate::query::{cast_instance, prepare_segment};
use crate::{CollisionPack, Hit, HitKind, RayOptions};

#[derive(Debug, Default)]
pub struct CollisionWorld {
    grid: Option<Grid>,
    packs: BTreeMap<u32, CollisionPack>,
    /// Packs the grid does not index (a mod's static collision outside the install's grid). Every query tests them, after
    /// the grid's candidates, since no grid cell names them.
    ungridded: Vec<CollisionPack>,
    revision: u64,
    /// The grid's candidate list of the last cast, kept so casts after warm-up do not allocate. A cast takes the
    /// buffer out and puts it back, so a predicate that casts again never meets a live borrow.
    candidates: RefCell<Vec<InstanceRef>>,
}

impl CollisionWorld {
    /// A world that picks candidates with `grid`, or tests every loaded instance without one.
    pub fn new(grid: Option<Grid>) -> Self {
        Self { grid, packs: BTreeMap::new(), ungridded: Vec::new(), revision: 0, candidates: RefCell::default() }
    }

    /// Adds (or replaces) a section's pack, applying the game's post-load exclusion flags.
    pub fn insert(&mut self, mut pack: CollisionPack) {
        pack.apply_group_exclusion();
        self.packs.insert(pack.section, pack);
        self.revision = self.revision.wrapping_add(1);
    }

    /// Adds a pack outside the grid: every ray cast and ground query tests it. The list is small, so it is tested in turn.
    pub fn insert_ungridded(&mut self, mut pack: CollisionPack) {
        pack.apply_group_exclusion();
        self.ungridded.push(pack);
        self.revision = self.revision.wrapping_add(1);
    }

    /// Removes the first ungridded pack of `section`, if there is one.
    pub fn remove_ungridded(&mut self, section: u32) -> Option<CollisionPack> {
        let at = self.ungridded.iter().position(|pack| pack.section == section)?;
        self.revision = self.revision.wrapping_add(1);
        Some(self.ungridded.remove(at))
    }

    pub fn remove(&mut self, section: u32) -> Option<CollisionPack> {
        let removed = self.packs.remove(&section);
        if removed.is_some() {
            self.revision = self.revision.wrapping_add(1);
        }
        removed
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

    pub(crate) fn packs(&self) -> impl Iterator<Item = &CollisionPack> {
        self.packs.values()
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// Takes the candidate buffer out for a cast. Pair with [`Self::return_candidates`]; a cast that finds it
    /// already taken (a predicate casting again) gets an empty buffer and allocates its own.
    pub(crate) fn take_candidates(&self) -> Vec<InstanceRef> {
        self.candidates.take()
    }

    /// Puts a buffer from [`Self::take_candidates`] back so the next cast reuses its capacity.
    pub(crate) fn return_candidates(&self, refs: Vec<InstanceRef>) {
        *self.candidates.borrow_mut() = refs;
    }

    /// Whether every instance referenced by the grid near `point` has a resident pack.
    /// Includes all cells touched by the xz square, so missing adjacent collision is detected
    /// before a body crosses a cell edge. `None` means there is no grid to establish coverage.
    pub fn resident_near(&self, point: Vec3, radius: f32) -> Option<bool> {
        let grid = self.grid.as_ref()?;
        if !point.into_iter().all(f32::is_finite) || !radius.is_finite() || radius < 0.0 {
            return Some(false);
        }
        let (r0, c0) = grid.cell_of(point[0] - radius, point[2] - radius);
        let (r1, c1) = grid.cell_of(point[0] + radius, point[2] + radius);
        Some((r0..=r1).all(|row| {
            (c0..=c1)
                .filter_map(|col| grid.node(grid.node_index(row, col)))
                .all(|node| node.instances.iter().all(|reference| self.packs.contains_key(&reference.section())))
        }))
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
                let mut refs = self.candidates.take();
                grid.instances_along_into(seg.0, seg.1, &mut refs);
                for r in &refs {
                    if let Some(pack) = self.packs.get(&r.section()) {
                        test(pack, r.index());
                    }
                }
                *self.candidates.borrow_mut() = refs;
            }
            None => {
                for pack in self.packs.values() {
                    for index in 0..pack.instances.len() {
                        test(pack, index);
                    }
                }
            }
        }
        for pack in &self.ungridded {
            for index in 0..pack.instances.len() {
                test(pack, index);
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
