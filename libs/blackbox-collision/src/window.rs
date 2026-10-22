//! Candidate sets: the instances of a block of grid cells, gathered once and tested by many short rays.
//!
//! A car casts about sixteen short rays per tick from within a few metres of one point. Walking the grid for each ray
//! sorts and de-duplicates the same candidates sixteen times. A [`CandidateSet`] holds the sorted, de-duplicated
//! instances of a block of cells, each tagged with the cells of the block it is listed in. A ray finds the cells it
//! touches (the grid's own slab test), keeps the instances listed in any of them, and tests those in the same sorted
//! order the per-ray walk would, so the nearest hit, ties and the order a predicate sees candidates are all unchanged.
//! A window a ray casts through also keeps its instances' ray-independent values between rays (`crate::face_cache`).

use crate::face_cache::FaceCache;
use crate::grid::{Grid, InstanceRef};
use crate::math::Vec3;
use crate::query::{Subject, cast_subject, prepare_segment};
use crate::world::{CollisionWorld, cache, keep_nearer};
use crate::{Hit, RayOptions};

/// Cells a set can span: one bit of an instance's cell mask each.
const MAX_SET_CELLS: u32 = 64;
/// How far past a ray the automatic window reaches, metres: enough that the rest of a car's probes fall inside it.
const WINDOW_PAD: f32 = 4.0;
/// The automatic window is built only when its block is at most this many cells; longer rays walk the grid.
const WINDOW_MAX_CELLS: u32 = 4;

#[derive(Debug, Clone, Copy)]
struct Entry {
    reference: InstanceRef,
    /// Index of the instance's pack in the world's pack list.
    slot: u32,
    /// Bit `(row - row0) * width + (col - col0)` is set for every cell of the block that lists the instance.
    cells: u64,
}

/// The instances of a block of grid cells, valid for one [`CollisionWorld`] revision. Fill it with
/// [`CollisionWorld::candidates_in`] and test rays against it with [`CollisionWorld::ray_cast_candidates`]; a set that
/// is empty, stale or does not cover a ray is never wrong, the cast falls back to the ordinary path.
#[derive(Debug, Clone, Default)]
pub struct CandidateSet {
    revision: u64,
    /// First row, first column, last row, last column (inclusive) of the gathered cells.
    block: Option<[u32; 4]>,
    entries: Vec<Entry>,
    /// The ray-independent values of its entries that rays cast through the set as a window keep (`crate::face_cache`).
    faces: FaceCache,
}

impl CandidateSet {
    /// Forgets the gathered cells; the allocation stays for the next gather.
    pub fn clear(&mut self) {
        self.block = None;
        self.entries.clear();
        self.faces.reset(0);
    }

    /// Candidates gathered (distinct, resident instances).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Triangles the set keeps for its rays, for tests that the keep is in use.
    #[cfg(test)]
    pub(crate) fn kept_triangles(&self) -> usize {
        self.faces.triangle_count()
    }

    /// The mask of the set's cells a segment touches, or `None` when the set is stale or the segment's cells are not
    /// all inside the set's block.
    fn mask_for(&self, grid: &Grid, revision: u64, (a, b): (Vec3, Vec3)) -> Option<u64> {
        let [row0, col0, row1, col1] = self.block?;
        if self.revision != revision || !finite_segment(a, b) {
            return None;
        }
        let (r0, c0) = grid.cell_of(a[0].min(b[0]), a[2].min(b[2]));
        let (r1, c1) = grid.cell_of(a[0].max(b[0]), a[2].max(b[2]));
        if r0 < row0 || c0 < col0 || r1 > row1 || c1 > col1 {
            return None;
        }
        let width = col1 - col0 + 1;
        let mut mask = 0u64;
        for row in r0..=r1 {
            for col in c0..=c1 {
                if grid.node(grid.node_index(row, col)).is_some() && grid.segment_touches(a, b, row, col) {
                    mask |= 1 << ((row - row0) * width + (col - col0));
                }
            }
        }
        Some(mask)
    }
}

fn finite_segment(a: Vec3, b: Vec3) -> bool {
    a.into_iter().chain(b).all(f32::is_finite)
}

impl CollisionWorld {
    /// Gathers into `out` the instances of the grid cells that the xz extent of the box `min`-`max` touches, so many rays
    /// inside the box can share the work ([`Self::ray_cast_candidates`]). Returns false and leaves `out` empty when
    /// there is no grid, the box is not finite, or it spans more than 64 cells.
    pub fn candidates_in(&self, min: Vec3, max: Vec3, out: &mut CandidateSet) -> bool {
        out.clear();
        let Some(grid) = &self.grid else { return false };
        let Some(block) = block_of(grid, min, max, MAX_SET_CELLS, 0.0) else { return false };
        self.gather(grid, block, out);
        true
    }

    /// The nearest hit of the segment `from`-`to`, as [`Self::ray_cast`], testing only the instances of `set` that the
    /// segment's cells list. Falls back to the ordinary cast when `set` was gathered at another revision or does not
    /// cover the segment's cells.
    pub fn ray_cast_candidates(&self, from: Vec3, to: Vec3, opts: &RayOptions, set: &CandidateSet) -> Option<Hit> {
        self.ray_cast_candidates_filtered(from, to, opts, set, |_| true)
    }

    /// [`Self::ray_cast_candidates`] with the predicate of [`Self::ray_cast_filtered`].
    pub fn ray_cast_candidates_filtered(
        &self,
        from: Vec3,
        to: Vec3,
        opts: &RayOptions,
        set: &CandidateSet,
        mut accept: impl FnMut(&Hit) -> bool,
    ) -> Option<Hit> {
        let seg = prepare_segment(from, to)?;
        let mask = self.grid.as_ref().and_then(|grid| set.mask_for(grid, self.revision, seg));
        let Some(mask) = mask else { return self.ray_cast_filtered(from, to, opts, accept) };
        self.cast_in(set, mask, seg, opts, &mut accept)
    }

    /// The nearest hit of the segment `from`-`to`, as [`Self::ray_cast`], keeping the candidate window in `window`
    /// instead of the world's own. A thread that casts while others do passes its own window, so the casts neither
    /// wait for each other nor rebuild each other's window; the hits are the same as [`Self::ray_cast`]'s.
    pub fn ray_cast_in(&self, from: Vec3, to: Vec3, opts: &RayOptions, window: &mut CandidateSet) -> Option<Hit> {
        let seg = prepare_segment(from, to)?;
        let mut accept = |_: &Hit| true;
        if let Some(result) = self.cast_windowed_in(window, seg, opts, &mut accept) {
            return result;
        }
        self.cast_walk(seg, opts, &mut accept)
    }

    /// The cast of a short ray against the world's automatic window, rebuilt around the ray when it does not cover it.
    /// `None` when the window does not apply (no grid, a non-finite or long ray) and the caller walks the grid.
    pub(crate) fn cast_windowed(
        &self,
        seg: (Vec3, Vec3),
        opts: &RayOptions,
        accept: &mut impl FnMut(&Hit) -> bool,
    ) -> Option<Option<Hit>> {
        self.grid.as_ref()?;
        // Taken out for the cast, so a predicate that casts again (or another thread) finds an empty window and
        // builds its own.
        let mut window = cache(&self.window).map(|mut kept| std::mem::take(&mut *kept)).unwrap_or_default();
        let result = self.cast_windowed_in(&mut window, seg, opts, accept);
        if let Some(mut kept) = cache(&self.window) {
            *kept = window;
        }
        result
    }

    /// [`Self::cast_windowed`] with the caller's window.
    fn cast_windowed_in(
        &self,
        window: &mut CandidateSet,
        seg: (Vec3, Vec3),
        opts: &RayOptions,
        accept: &mut impl FnMut(&Hit) -> bool,
    ) -> Option<Option<Hit>> {
        let grid = self.grid.as_ref()?;
        if !finite_segment(seg.0, seg.1) {
            return None;
        }
        let mask = window.mask_for(grid, self.revision, seg).or_else(|| self.rebuild_window(grid, window, seg))?;
        Some(self.cast_kept(window, mask, seg, opts, accept))
    }

    /// Gathers `window` around `seg` and returns the segment's mask, or `None` when the block would be too large.
    fn rebuild_window(&self, grid: &Grid, window: &mut CandidateSet, seg: (Vec3, Vec3)) -> Option<u64> {
        let (min, max) = (min_of(seg), max_of(seg));
        let block = block_of(grid, min, max, WINDOW_MAX_CELLS, WINDOW_PAD)?;
        self.gather(grid, block, window);
        window.mask_for(grid, self.revision, seg)
    }

    fn gather(&self, grid: &Grid, block: [u32; 4], out: &mut CandidateSet) {
        out.entries.clear();
        out.revision = self.revision;
        out.block = Some(block);
        let [row0, col0, row1, col1] = block;
        let width = col1 - col0 + 1;
        for row in row0..=row1 {
            for col in col0..=col1 {
                let Some(node) = grid.node(grid.node_index(row, col)) else { continue };
                let cells = 1u64 << ((row - row0) * width + (col - col0));
                out.entries.extend(node.instances.iter().filter_map(|&reference| {
                    let slot = self.slot_of(reference.section())?;
                    Some(Entry { reference, slot, cells })
                }));
            }
        }
        out.entries.sort_unstable_by_key(|entry| entry.reference);
        // A duplicate is the same instance listed in another cell of the block: keep one, with both cells.
        out.entries.dedup_by(|later, earlier| {
            if later.reference != earlier.reference {
                return false;
            }
            earlier.cells |= later.cells;
            true
        });
        out.faces.reset(out.entries.len());
    }

    /// Tests the instances of `set` listed in the cells of `mask`, in instance order, then the ungridded packs.
    fn cast_in(
        &self,
        set: &CandidateSet,
        mask: u64,
        seg: (Vec3, Vec3),
        opts: &RayOptions,
        accept: &mut impl FnMut(&Hit) -> bool,
    ) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        for entry in set.entries.iter().filter(|entry| entry.cells & mask != 0) {
            let Some(pack) = self.packs.get(entry.slot as usize) else { continue };
            self.test_instance(pack, entry.reference.index(), seg, opts, accept, &mut best);
        }
        self.test_ungridded(seg, opts, accept, &mut best);
        best
    }

    /// [`Self::cast_in`] with the values the window keeps between rays (`crate::face_cache`): the same instances in the
    /// same order through the same instance cast, so the same hits.
    fn cast_kept(
        &self,
        window: &mut CandidateSet,
        mask: u64,
        seg: (Vec3, Vec3),
        opts: &RayOptions,
        accept: &mut impl FnMut(&Hit) -> bool,
    ) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        let CandidateSet { entries, faces, .. } = window;
        for (at, entry) in entries.iter().enumerate() {
            if entry.cells & mask == 0 {
                continue;
            }
            let Some(pack) = self.packs.get(entry.slot as usize) else { continue };
            let index = entry.reference.index();
            let (Some(inst), Some(article)) = (pack.instances.get(index), pack.article_of(index)) else { continue };
            let Some((broad, mut strips)) = faces.instance(at, inst, article) else {
                self.test_instance(pack, index, seg, opts, accept, &mut best);
                continue;
            };
            let subject = Subject { inst, article, groups: &self.enabled_groups, identity: (pack.section, index) };
            let Some(hit) = cast_subject(&subject, |_| broad, &mut strips, seg, opts, accept) else { continue };
            keep_nearer(&mut best, hit, seg.0);
        }
        self.test_ungridded(seg, opts, accept, &mut best);
        best
    }

    /// [`Self::ray_cast_filtered`] without any candidate reuse: what the window and candidate-set paths must equal.
    #[cfg(test)]
    pub(crate) fn ray_cast_walk(
        &self,
        from: Vec3,
        to: Vec3,
        opts: &RayOptions,
        mut accept: impl FnMut(&Hit) -> bool,
    ) -> Option<Hit> {
        let seg = prepare_segment(from, to)?;
        self.cast_walk(seg, opts, &mut accept)
    }
}

fn min_of((a, b): (Vec3, Vec3)) -> Vec3 {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])]
}

fn max_of((a, b): (Vec3, Vec3)) -> Vec3 {
    [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])]
}

/// The inclusive cell block of the xz extent of `min`-`max` grown by `pad`, when it is finite and has at most
/// `max_cells` cells.
fn block_of(grid: &Grid, min: Vec3, max: Vec3, max_cells: u32, pad: f32) -> Option<[u32; 4]> {
    if !finite_segment(min, max) {
        return None;
    }
    let (r0, c0) = grid.cell_of(min[0].min(max[0]) - pad, min[2].min(max[2]) - pad);
    let (r1, c1) = grid.cell_of(min[0].max(max[0]) + pad, min[2].max(max[2]) + pad);
    let cells = (r1 - r0 + 1).checked_mul(c1 - c0 + 1)?;
    (cells <= max_cells).then_some([r0, c0, r1, c1])
}
