//! Record layouts of the `VisibleSectionManager` tables.

/// `VisibleSectionManagerInfo`: the far-section offset and the region's drivable sections.
#[derive(Debug, Clone)]
pub struct InfoLayout {
    /// `i32` LODOffset.
    pub lod_offset: usize,
    /// `i32` count of `DrivableSectionsInRegion`.
    pub region_count: usize,
    /// `i16[region_max]`.
    pub region_list: usize,
    pub region_max: usize,
}

/// `VisibleSectionBoundary`: `points + point_len × NumPoints` bytes.
#[derive(Debug, Clone)]
pub struct BoundaryLayout {
    /// `i16`.
    pub section: usize,
    /// `i8`.
    pub num_points: usize,
    /// `i8`.
    pub panorama: usize,
    /// `f32` x, y.
    pub bbox_min: usize,
    pub bbox_max: usize,
    pub centre: usize,
    /// `f32` x, y per point.
    pub points: usize,
    pub point_len: usize,
}

/// `DrivableScenerySection`: `list + 2 × MaxVisibleSections + trailing` bytes.
#[derive(Debug, Clone)]
pub struct DrivableLayout {
    /// `i16`.
    pub section: usize,
    /// `i8` record capacity.
    pub max_visible: usize,
    /// `i16`.
    pub num_visible: usize,
    /// `i16[max_visible]`.
    pub list: usize,
    /// Padding after the list.
    pub trailing: usize,
}

/// `LoadingSection`: fixed-size records.
#[derive(Debug, Clone)]
pub struct LoadingLayout {
    pub len: usize,
    /// `char[name_len]`.
    pub name: usize,
    pub name_len: usize,
    /// `i8`.
    pub default_flag: usize,
    /// `i16` count, then `i16[drivable_max]`.
    pub num_drivable: usize,
    pub drivable: usize,
    pub drivable_max: usize,
    /// `i16` count, then `i16[extra_max]`.
    pub num_extra: usize,
    pub extra: usize,
    pub extra_max: usize,
}

/// The `VisibleSectionManager` tables of one game.
#[derive(Debug, Clone)]
pub struct VisibleLayout {
    pub used_by: &'static str,
    pub info: InfoLayout,
    pub boundary: BoundaryLayout,
    pub drivable: DrivableLayout,
    pub loading: LoadingLayout,
}
