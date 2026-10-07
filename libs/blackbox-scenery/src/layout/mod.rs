//! Scenery record layouts per game.

mod most_wanted;

pub use most_wanted::MOST_WANTED;

/// `SceneryInfo`: which solid (per LOD) a scenery object uses.
#[derive(Debug, Clone)]
pub struct InfoLayout {
    pub len: usize,
    /// Payload alignment (1 = none).
    pub align: usize,
    /// `char[name_len]`.
    pub name: usize,
    pub name_len: usize,
    /// `u32[lods]`: `bStringHash` of the solid for each LOD, highest first.
    pub solid_keys: usize,
    pub lods: usize,
    pub radius: usize,
    pub hierarchy_hash: usize,
}

/// `SceneryInstance`: one placed copy.
#[derive(Debug, Clone)]
pub struct InstanceLayout {
    pub len: usize,
    pub align: usize,
    pub bbox_min: usize,
    pub bbox_max: usize,
    pub exclude_flags: usize,
    pub preculler_index: usize,
    pub lighting_context: usize,
    pub position: usize,
    /// 9 × `i16`, row-major 3×3 (rows are the transformed x, y, z axes; may include scale).
    pub rotation: usize,
    /// Divide the `i16` rotation values by this.
    pub rotation_scale: f32,
    pub info_index: usize,
}

/// How exclude flags hide instances per view (`docs/specs/scenery-visibility.md`).
#[derive(Debug, Clone)]
pub struct VisibilityRules {
    /// Instance flag bits that mean "also draw in this view" rather than "never draw".
    pub inverted_bits: u32,
    /// View flags of the normal player camera.
    pub player_view: u32,
}

/// Constants of the LOD choice (`docs/specs/scenery-lod.md`).
#[derive(Debug, Clone)]
pub struct LodRules {
    /// Added to the info radius before projecting.
    pub radius_pad: f32,
    /// Below this projected size, skip without further work.
    pub min_size: i32,
    /// Instance flag that adds `boost` pixels.
    pub boost_flag: u32,
    pub boost: i32,
    /// Minimum projected size to draw anything.
    pub draw_threshold: i32,
    /// Below this polygon count, the density test is skipped.
    pub poly_threshold: u16,
    pub density_floor: f32,
    pub density_threshold: f32,
    pub detailed_slot: usize,
    pub coarse_slot: usize,
}

#[derive(Debug, Clone)]
pub struct SceneryLayout {
    pub used_by: &'static str,
    pub info: InfoLayout,
    pub instance: InstanceLayout,
    /// Offset of the u32 section number in `ScenerySectionHeader`.
    pub header_section_number: usize,
    pub visibility: VisibilityRules,
    pub lod: LodRules,
}
