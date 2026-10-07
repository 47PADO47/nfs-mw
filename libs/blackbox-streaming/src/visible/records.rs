//! The records of the `VisibleSectionManager` tables.

/// A zone or a non-drivable section's area: a closed 2D polygon in world x, y.
#[derive(Debug, Clone, PartialEq)]
pub struct Boundary {
    pub section: i16,
    /// Set on the boundaries of panorama (backdrop) sections.
    pub panorama: bool,
    pub bbox_min: [f32; 2],
    pub bbox_max: [f32; 2],
    pub centre: [f32; 2],
    pub points: Vec<[f32; 2]>,
}

/// A drivable section and the sections visible from it.
#[derive(Debug, Clone, PartialEq)]
pub struct DrivableSection {
    pub section: i16,
    /// Sorted section numbers; some may have no entry in the streaming index.
    pub visible: Vec<i16>,
}

/// Neighbouring drivable sections that load as one, plus extra sections.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadingSection {
    pub name: String,
    pub default: bool,
    pub drivable: Vec<i16>,
    pub extra: Vec<i16>,
}
