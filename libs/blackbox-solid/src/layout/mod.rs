//! Record layouts per `SolidInfo` version.
//!
//! Each supported version gets one file with a [`SolidLayout`] constant. Add a
//! game by adding its version here once its layout is documented in
//! `docs/formats/models.md`.

mod v16;

/// Field offsets inside `SolidInfo`.
#[derive(Debug, Clone)]
pub struct InfoLayout {
    /// Alignment of the chunk payload.
    pub align: usize,
    pub version: usize,
    pub flags: usize,
    pub name_hash: usize,
    pub bounds_min: usize,
    pub bounds_max: usize,
    pub transform: usize,
    /// `i16` polygon count.
    pub num_polys: usize,
    /// `f32` polygon density, used by scenery LOD selection.
    pub density: usize,
    /// NUL-terminated name.
    pub name: usize,
}

/// Field offsets inside one shading-group record.
#[derive(Debug, Clone)]
pub struct GroupLayout {
    pub len: usize,
    pub bounds_min: usize,
    pub bounds_max: usize,
    /// Five u8 texture slots, then the u8 light-material index.
    pub texture_slots: usize,
    pub effect_id: usize,
    pub flags: usize,
    pub num_vertices: usize,
    pub num_triangles: usize,
    pub first_index: usize,
    pub num_indices: usize,
}

/// Field offsets inside one position-marker record (`SolidMarkers`).
#[derive(Debug, Clone)]
pub struct MarkerLayout {
    pub len: usize,
    /// Alignment of the chunk payload.
    pub align: usize,
    pub name_hash: usize,
    pub int_param: usize,
    /// Two `f32`s.
    pub float_params: usize,
    /// 16 `f32`s, row-major.
    pub matrix: usize,
}

/// Everything version-specific about a solid.
#[derive(Debug, Clone)]
pub struct SolidLayout {
    /// The `SolidInfo` version byte.
    pub version: u8,
    /// Which game(s) are known to use it, for messages and docs.
    pub used_by: &'static str,
    pub info: InfoLayout,
    pub group: GroupLayout,
    pub marker: MarkerLayout,
    pub groups_align: usize,
    pub indices_align: usize,
    pub vertices_align: usize,
}

/// All supported layouts.
pub const LAYOUTS: &[&SolidLayout] = &[&v16::V16];

/// Offset of the version byte in `SolidInfo`, shared by every known layout.
pub const VERSION_OFFSET: usize = 0x0C;

/// The layout for a `SolidInfo` version byte.
pub fn for_version(version: u8) -> Option<&'static SolidLayout> {
    LAYOUTS.iter().copied().find(|l| l.version == version)
}
