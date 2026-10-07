//! Which zone a point is in, and what a zone loads and draws
//! (`docs/specs/visible-sections.md`).

use super::{DrivableSection, LoadingSection, VisibleSections};

/// How far outside every boundary a point may be and still count as in the nearest zone.
pub const BOUNDARY_TOLERANCE: f32 = 0.1;

impl VisibleSections {
    /// The drivable section whose boundary contains the point, or whose boundary is less than
    /// [`BOUNDARY_TOLERANCE`] away; `None` elsewhere (the game's zone 0).
    pub fn drivable_at(&self, p: [f32; 2]) -> Option<&DrivableSection> {
        let (section, distance) = self.closest_drivable(p, |_| true)?;
        (distance < BOUNDARY_TOLERANCE).then_some(section)
    }

    /// The drivable section accepted by `filter` whose boundary is nearest to the point (0 when
    /// inside), with that distance. Ties go to the lower section number.
    pub fn closest_drivable(&self, p: [f32; 2], filter: impl Fn(i16) -> bool) -> Option<(&DrivableSection, f32)> {
        let numbering = self.numbering();
        let mut best: Option<(i16, f32)> = None;
        for b in self.boundaries.iter().filter(|b| numbering.is_drivable(b.section) && filter(b.section)) {
            if b.contains(p) {
                best = Some((b.section, 0.0));
                break;
            }
            let d = b.distance_outside(p);
            if best.is_none_or(|(s, bd)| d < bd || (d == bd && b.section < s)) {
                best = Some((b.section, d));
            }
        }
        let (section, distance) = best?;
        Some((self.drivable(section)?, distance))
    }

    /// The first loading section that groups `zone` with other drivable sections.
    pub fn loading_section_of(&self, zone: i16) -> Option<&LoadingSection> {
        self.loading.iter().find(|l| l.drivable.contains(&zone))
    }

    /// Sections to keep loaded while the player is in `zone`, besides the always-loaded shared
    /// sets: the zone's loading section (the visible lists of all its drivable sections, then its
    /// extras, with the far counterpart of a drivable extra), or else the zone's own visible list.
    /// No duplicates; numbers may lack an entry in the streaming index.
    pub fn sections_to_load(&self, zone: i16) -> Vec<i16> {
        let mut out = Vec::new();
        let mut add = |s: i16| {
            if !out.contains(&s) {
                out.push(s);
            }
        };
        let Some(loading) = self.loading_section_of(zone) else {
            self.sections_to_draw(zone).iter().copied().for_each(add);
            return out;
        };
        for d in loading.drivable.iter().filter_map(|&d| self.drivable(d)) {
            d.visible.iter().copied().for_each(&mut add);
        }
        let numbering = self.numbering();
        for &extra in &loading.extra {
            add(extra);
            if numbering.is_drivable(extra) {
                add(numbering.far_of(extra));
            }
        }
        out
    }

    /// Sections a view in `zone` draws, besides the shared `Z` sections: the zone's visible list.
    pub fn sections_to_draw(&self, zone: i16) -> &[i16] {
        self.drivable(zone).map_or(&[], |d| d.visible.as_slice())
    }
}
