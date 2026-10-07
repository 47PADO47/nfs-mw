//! The `VisibleSectionManager` tables: zones (drivable sections with a 2D
//! boundary), the sections each zone loads and draws, and loading sections.
//! Layout: `docs/formats/maps.md`; behaviour: `docs/specs/visible-sections.md`.

mod numbering;
mod reader;
mod records;
#[cfg(test)]
mod tests;

pub use numbering::Numbering;
pub use reader::read_visible_sections;
pub use records::{Boundary, DrivableSection, LoadingSection};

/// Everything in a track's `VisibleSectionManager` chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleSections {
    /// Splits drivable section numbers (`1..lod_offset`) from their far counterparts.
    pub lod_offset: i16,
    /// `DrivableSectionsInRegion`.
    pub region: Vec<i16>,
    pub boundaries: Vec<Boundary>,
    pub drivable: Vec<DrivableSection>,
    pub loading: Vec<LoadingSection>,
}

impl VisibleSections {
    pub fn numbering(&self) -> Numbering {
        Numbering { lod_offset: self.lod_offset }
    }

    /// The boundary of a section, if it has one.
    pub fn boundary(&self, section: i16) -> Option<&Boundary> {
        self.boundaries.iter().find(|b| b.section == section)
    }

    /// The visible list of a drivable section.
    pub fn drivable(&self, section: i16) -> Option<&DrivableSection> {
        self.drivable.iter().find(|d| d.section == section)
    }

    /// Whether `DrivableSectionsInRegion` lists the section.
    pub fn in_region(&self, section: i16) -> bool {
        self.region.contains(&section)
    }
}
