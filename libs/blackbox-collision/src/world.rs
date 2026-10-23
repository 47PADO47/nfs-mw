//! A set of loaded collision packs plus the grid that finds them: the engine's `WCollisionAssets`
//! and `WCollisionMgr` in one value.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, TryLockError};

use crate::grid::{Grid, InstanceRef};
use crate::groups::GroupSet;
use crate::math::{Vec3, dot, sub};
use crate::query::{cast_instance, prepare_segment};
use crate::window::CandidateSet;
use crate::{CollisionPack, Hit, HitKind, RayOptions};

/// Most `resident_near` answers kept per revision; a streaming world asks about a handful of cell blocks.
const RESIDENT_CACHE_LIMIT: usize = 1024;

/// `resident_near` answers for one world revision, keyed by the grid cell block the query touches (the answer is a
/// function of that block and of which packs are loaded, nothing else).
#[derive(Debug, Default)]
struct ResidentCache {
    revision: u64,
    blocks: BTreeMap<[u32; 4], bool>,
}

#[derive(Debug, Default)]
pub struct CollisionWorld {
    pub(crate) grid: Option<Grid>,
    /// The loaded packs, sorted by section so unindexed iteration keeps the section order.
    pub(crate) packs: Vec<CollisionPack>,
    /// Section (a `u16` in the grid's references) to its index in `packs` plus one; 0 is not loaded. Rebuilt whenever
    /// a pack is inserted or removed, so a grid candidate finds its pack without a tree walk.
    slots: Vec<u32>,
    /// Packs the grid does not index (a mod's static collision outside the install's grid). Every query tests them, after
    /// the grid's candidates, since no grid cell names them.
    pub(crate) ungridded: Vec<CollisionPack>,
    pub(crate) revision: u64,
    /// The grid's candidate list of the last cast, kept so casts after warm-up do not allocate. A cast takes the
    /// buffer out and puts it back, so a predicate that casts again never meets a live borrow.
    ///
    /// The caches are behind locks only so the world can be shared by threads that cast at once (each with its own
    /// window, [`Self::ray_cast_in`]). A cast never waits for one: when another thread holds it, the cast runs without
    /// the cache, which gives the same answer (the `cache` helper below).
    candidates: Mutex<Vec<InstanceRef>>,
    /// The candidate set the last short ray cast used. A car's probes and corner rays, and the cars near it, fall in
    /// the same few cells, so the next cast reuses it when it covers the ray and the revision still matches.
    pub(crate) window: Mutex<CandidateSet>,
    resident: Mutex<ResidentCache>,
    /// Scenery groups (`group != 0`) that are in the world: their instances collide when a query skips groups
    /// ([`RayOptions::skip_groups`]). Kept here so it survives packs being inserted and removed.
    pub(crate) enabled_groups: GroupSet,
    /// Scenery groups whose barriers the caller treats as walls from both sides ([`Self::set_group_two_sided`]).
    two_sided_groups: GroupSet,
}

impl CollisionWorld {
    /// A world that picks candidates with `grid`, or tests every loaded instance without one.
    pub fn new(grid: Option<Grid>) -> Self {
        Self {
            grid,
            packs: Vec::new(),
            slots: Vec::new(),
            ungridded: Vec::new(),
            revision: 0,
            candidates: Mutex::default(),
            window: Mutex::default(),
            resident: Mutex::default(),
            enabled_groups: GroupSet::default(),
            two_sided_groups: GroupSet::default(),
        }
    }

    /// Switches scenery group `group` in or out of the world. While it is in, queries that skip groups test its
    /// instances and ignore their baked group-exclusion bits; while it is out they are skipped. Group 0 is not a group.
    pub fn set_group_enabled(&mut self, group: u16, enabled: bool) {
        let changed = if enabled { self.enabled_groups.insert(group) } else { self.enabled_groups.remove(group) };
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Whether scenery group `group` is in the world (see [`Self::set_group_enabled`]).
    pub fn group_enabled(&self, group: u16) -> bool {
        self.enabled_groups.contains(group)
    }

    /// Marks scenery group `group`'s barriers as walls from both sides, or from their front only again. Queries return
    /// the same hits either way: the mark is for callers that skip back-facing barriers ([`Hit::front_facing`]), which
    /// ask [`Self::group_two_sided`] with the hit's [`Hit::group`]. Group 0 is not a group and is never two-sided.
    pub fn set_group_two_sided(&mut self, group: u16, two_sided: bool) {
        if group == 0 {
            return;
        }
        match two_sided {
            true => self.two_sided_groups.insert(group),
            false => self.two_sided_groups.remove(group),
        };
    }

    /// Whether scenery group `group`'s barriers are walls from both sides (see [`Self::set_group_two_sided`]).
    pub fn group_two_sided(&self, group: u16) -> bool {
        group != 0 && self.two_sided_groups.contains(group)
    }

    /// Adds (or replaces) a section's pack, applying the game's post-load exclusion flags.
    pub fn insert(&mut self, mut pack: CollisionPack) {
        pack.apply_group_exclusion();
        let section = pack.section;
        match self.packs.binary_search_by_key(&section, |p| p.section) {
            Ok(at) => self.packs[at] = pack,
            Err(at) => self.packs.insert(at, pack),
        }
        self.rebuild_slots();
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
        let at = self.packs.binary_search_by_key(&section, |p| p.section).ok()?;
        let removed = self.packs.remove(at);
        self.rebuild_slots();
        self.revision = self.revision.wrapping_add(1);
        Some(removed)
    }

    pub fn pack(&self, section: u32) -> Option<&CollisionPack> {
        if section <= u32::from(u16::MAX) {
            return self.packs.get(self.slot_of(section)? as usize);
        }
        let at = self.packs.binary_search_by_key(&section, |p| p.section).ok()?;
        self.packs.get(at)
    }

    /// Index in `packs` of the pack of `section`, for the sections a grid reference can name.
    pub(crate) fn slot_of(&self, section: u32) -> Option<u32> {
        self.slots.get(section as usize)?.checked_sub(1)
    }

    fn rebuild_slots(&mut self) {
        self.slots.clear();
        for (at, pack) in self.packs.iter().enumerate() {
            let Ok(section) = u16::try_from(pack.section) else { continue };
            let section = usize::from(section);
            if self.slots.len() <= section {
                self.slots.resize(section + 1, 0);
            }
            self.slots[section] = at as u32 + 1;
        }
    }

    pub fn grid(&self) -> Option<&Grid> {
        self.grid.as_ref()
    }

    pub fn pack_count(&self) -> usize {
        self.packs.len()
    }

    pub(crate) fn packs(&self) -> impl Iterator<Item = &CollisionPack> {
        self.packs.iter()
    }

    pub(crate) fn ungridded_packs(&self) -> impl Iterator<Item = &CollisionPack> {
        self.ungridded.iter()
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// Takes the candidate buffer out for a cast. Pair with [`Self::return_candidates`]; a cast that finds it
    /// already taken (a predicate casting again) gets an empty buffer and allocates its own.
    pub(crate) fn take_candidates(&self) -> Vec<InstanceRef> {
        cache(&self.candidates).map(|mut buffer| std::mem::take(&mut *buffer)).unwrap_or_default()
    }

    /// Puts a buffer from [`Self::take_candidates`] back so the next cast reuses its capacity.
    pub(crate) fn return_candidates(&self, refs: Vec<InstanceRef>) {
        if let Some(mut buffer) = cache(&self.candidates) {
            *buffer = refs;
        }
    }

    /// Whether every instance referenced by the grid near `point` has a resident pack.
    /// Includes all cells touched by the xz square, so missing adjacent collision is detected
    /// before a body crosses a cell edge. `None` means there is no grid to establish coverage.
    ///
    /// The answer depends only on the cell block the square touches and on which packs are loaded, so it is kept per
    /// world revision and block: a car asking every tick from the same cell costs one lookup.
    pub fn resident_near(&self, point: Vec3, radius: f32) -> Option<bool> {
        let grid = self.grid.as_ref()?;
        if !point.into_iter().all(f32::is_finite) || !radius.is_finite() || radius < 0.0 {
            return Some(false);
        }
        let (r0, c0) = grid.cell_of(point[0] - radius, point[2] - radius);
        let (r1, c1) = grid.cell_of(point[0] + radius, point[2] + radius);
        let block = [r0, c0, r1, c1];
        let Some(mut answers) = cache(&self.resident) else { return Some(self.block_resident(grid, block)) };
        if answers.revision != self.revision {
            answers.blocks.clear();
            answers.revision = self.revision;
        }
        if let Some(&ready) = answers.blocks.get(&block) {
            return Some(ready);
        }
        let ready = self.block_resident(grid, block);
        if answers.blocks.len() >= RESIDENT_CACHE_LIMIT {
            answers.blocks.clear();
        }
        answers.blocks.insert(block, ready);
        Some(ready)
    }

    /// Whether every instance the cells of `block` list has a resident pack: the uncached answer of `resident_near`.
    fn block_resident(&self, grid: &Grid, [r0, c0, r1, c1]: [u32; 4]) -> bool {
        (r0..=r1).all(|row| {
            (c0..=c1)
                .filter_map(|col| grid.node(grid.node_index(row, col)))
                .all(|node| node.instances.iter().all(|reference| self.slot_of(reference.section()).is_some()))
        })
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
        if let Some(result) = self.cast_windowed(seg, opts, &mut accept) {
            return result;
        }
        self.cast_walk(seg, opts, &mut accept)
    }

    /// [`Self::ray_cast_filtered`] by walking the grid cells along the ray for this one cast: the reference every other
    /// path must match bit for bit.
    pub(crate) fn cast_walk(
        &self,
        seg: (Vec3, Vec3),
        opts: &RayOptions,
        accept: &mut impl FnMut(&Hit) -> bool,
    ) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        match &self.grid {
            Some(grid) => {
                let mut refs = self.take_candidates();
                grid.instances_along_into(seg.0, seg.1, &mut refs);
                for r in &refs {
                    let Some(pack) = self.slot_of(r.section()).and_then(|slot| self.packs.get(slot as usize)) else {
                        continue;
                    };
                    self.test_instance(pack, r.index(), seg, opts, accept, &mut best);
                }
                self.return_candidates(refs);
            }
            None => {
                for pack in &self.packs {
                    for index in 0..pack.instances.len() {
                        self.test_instance(pack, index, seg, opts, accept, &mut best);
                    }
                }
            }
        }
        self.test_ungridded(seg, opts, accept, &mut best);
        best
    }

    /// The packs outside the grid, tested after the grid's candidates.
    pub(crate) fn test_ungridded(
        &self,
        seg: (Vec3, Vec3),
        opts: &RayOptions,
        accept: &mut impl FnMut(&Hit) -> bool,
        best: &mut Option<Hit>,
    ) {
        for pack in &self.ungridded {
            for index in 0..pack.instances.len() {
                self.test_instance(pack, index, seg, opts, accept, best);
            }
        }
    }

    /// Casts the prepared segment on one instance and keeps the hit if it beats `best`.
    pub(crate) fn test_instance(
        &self,
        pack: &CollisionPack,
        index: usize,
        seg: (Vec3, Vec3),
        opts: &RayOptions,
        accept: &mut impl FnMut(&Hit) -> bool,
        best: &mut Option<Hit>,
    ) {
        let (Some(inst), Some(article)) = (pack.instances.get(index), pack.article_of(index)) else { return };
        let identity = (pack.section, index);
        let Some(hit) = cast_instance(inst, article, seg, opts, &self.enabled_groups, identity, accept) else { return };
        keep_nearer(best, hit, seg.0);
    }

    /// The first surface met going down from `(x, top, z)` to `(x, bottom, z)`: ground height
    /// queries. Faces only.
    pub fn ground(&self, x: f32, z: f32, top: f32, bottom: f32) -> Option<Hit> {
        let opts = RayOptions { barriers: false, ..RayOptions::default() };
        self.ray_cast([x, top, z], [x, bottom, z], &opts)
    }
}

/// The cache behind `lock`, or `None` while another thread holds it: the caller then works without the cache. A cache
/// only saves work (every cached answer equals the uncached one), so a poisoned lock is still a usable cache.
pub(crate) fn cache<T>(lock: &Mutex<T>) -> Option<MutexGuard<'_, T>> {
    match lock.try_lock() {
        Ok(guard) => Some(guard),
        Err(TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    }
}

/// Keeps `hit` in `best` when it is nearer `start` ([`closer`]): how every cast picks among instances.
pub(crate) fn keep_nearer(best: &mut Option<Hit>, hit: Hit, start: Vec3) {
    if best.as_ref().is_none_or(|b| closer(&hit, b, start)) {
        *best = Some(hit);
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
