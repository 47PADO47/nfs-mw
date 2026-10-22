//! What a candidate window keeps of its instances between rays: the broad-phase values of each instance a ray has
//! reached and the decoded triangles of each strip a ray has entered.
//!
//! A car's corner rays and probes fall on the same few instances and strips, tick after tick. The plain cast decodes
//! a strip's packed vertices and recomputes an instance's centre for every ray. Both are functions of the pack alone,
//! so the window computes them once, with the plain cast's own functions ([`Broad::of`], [`FaceTriangle::of`]), and
//! the rays test the kept values in the plain cast's own code ([`crate::query::cast_subject`]): the same operations on
//! the same bits, so every hit is the plain cast's, bit for bit. Nothing ray dependent (the segment in the instance's
//! space, the option filters, the scenery groups) is kept. The keep is cleared whenever the window is gathered again,
//! which is also the only way a window follows a new world revision.

use crate::query::{Broad, FaceTriangle, StripFaces};
use crate::{Article, Instance, Strip};

/// An instance a ray of the window has reached.
#[derive(Debug, Clone, Copy)]
struct Kept {
    broad: Broad,
    /// Its article's first strip in `FaceCache::strips`.
    first_strip: usize,
    strip_count: usize,
}

/// The kept values of one window, indexed like its entries. Filled as rays reach instances and strips.
#[derive(Debug, Clone, Default)]
pub(crate) struct FaceCache {
    /// Per entry of the window: its index in `instances` plus one, 0 until a ray reaches it.
    by_entry: Vec<u32>,
    instances: Vec<Kept>,
    /// Per strip of each kept instance's article: its first triangle in `triangles` plus one, 0 until a ray enters it.
    strips: Vec<u32>,
    triangles: Vec<FaceTriangle>,
}

impl FaceCache {
    /// Forgets everything kept, for a window of `entries` entries. The allocations stay.
    pub(crate) fn reset(&mut self, entries: usize) {
        self.by_entry.clear();
        self.by_entry.resize(entries, 0);
        self.instances.clear();
        self.strips.clear();
        self.triangles.clear();
    }

    /// The broad-phase values of entry `entry` (placing `inst`, with `article`) and its strips' triangles, kept from
    /// here on. `None` when the entry is not one of the window's (the caller then casts without the keep).
    pub(crate) fn instance(
        &mut self,
        entry: usize,
        inst: &Instance,
        article: &Article,
    ) -> Option<(Broad, KeptStrips<'_>)> {
        let slot = *self.by_entry.get(entry)?;
        let kept = match slot.checked_sub(1) {
            Some(at) => *self.instances.get(at as usize)?,
            None => self.keep(entry, inst, article)?,
        };
        let strips = self.strips.get_mut(kept.first_strip..kept.first_strip + kept.strip_count)?;
        Some((kept.broad, KeptStrips { strips, triangles: &mut self.triangles }))
    }

    fn keep(&mut self, entry: usize, inst: &Instance, article: &Article) -> Option<Kept> {
        let mark = u32::try_from(self.instances.len() + 1).ok()?;
        let kept = Kept { broad: Broad::of(inst), first_strip: self.strips.len(), strip_count: article.strips.len() };
        self.strips.resize(kept.first_strip + kept.strip_count, 0);
        self.instances.push(kept);
        self.by_entry[entry] = mark;
        Some(kept)
    }

    /// Triangles kept, for tests that the keep is in use.
    #[cfg(test)]
    pub(crate) fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
}

/// One kept instance's strips: each strip's triangles are decoded the first time a ray enters it.
pub(crate) struct KeptStrips<'a> {
    strips: &'a mut [u32],
    triangles: &'a mut Vec<FaceTriangle>,
}

impl StripFaces for KeptStrips<'_> {
    fn each_triangle(&mut self, index: usize, strip: &Strip, mut each: impl FnMut(&FaceTriangle)) {
        let Some(start) = self.start_of(index, strip) else {
            strip.triangles().for_each(|tri| each(&FaceTriangle::of(&tri)));
            return;
        };
        self.triangles[start..start + strip.triangle_count()].iter().for_each(each);
    }
}

impl KeptStrips<'_> {
    /// Where the triangles of strip `index` start in the kept list, decoding them when a ray first enters the strip.
    /// `None` when the strip cannot be kept.
    fn start_of(&mut self, index: usize, strip: &Strip) -> Option<usize> {
        let slot = self.strips.get_mut(index)?;
        if let Some(start) = slot.checked_sub(1) {
            return Some(start as usize);
        }
        let start = self.triangles.len();
        let mark = u32::try_from(start + 1).ok()?;
        self.triangles.extend(strip.triangles().map(|tri| FaceTriangle::of(&tri)));
        *slot = mark;
        Some(start)
    }
}
