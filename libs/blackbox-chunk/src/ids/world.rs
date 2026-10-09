//! Streamed worlds: `docs/formats/maps.md`.

/// `TrackStreamingSections`: the streaming index in the track metadata file.
pub const TRACK_STREAMING_SECTIONS: u32 = 0x0003_4110;
pub const SCENERY_SECTION: u32 = 0x8003_4100;
pub const SCENERY_SECTION_HEADER: u32 = 0x0003_4101;
pub const SCENERY_INFOS: u32 = 0x0003_4102;
pub const SCENERY_INSTANCES: u32 = 0x0003_4103;
pub const SCENERY_TREE_NODES: u32 = 0x0003_4105;
pub const SCENERY_OVERRIDE_HOOKS: u32 = 0x0003_4106;
pub const SCENERY_PRECULLER_INFOS: u32 = 0x0003_4107;
/// `VisibleSectionManager`: zones and what each one loads and draws.
pub const VISIBLE_SECTION_MANAGER: u32 = 0x8003_4150;
pub const VISIBLE_SECTION_MANAGER_INFO: u32 = 0x0003_4151;
pub const VISIBLE_SECTION_BOUNDARIES: u32 = 0x0003_4152;
pub const DRIVABLE_SCENERY_SECTIONS: u32 = 0x0003_4153;
pub const LOADING_SECTIONS: u32 = 0x0003_4155;
/// `TrackPathManager`: container of the track path zones and barriers (`docs/formats/road-network.md`).
pub const TRACK_PATH_MANAGER: u32 = 0x8003_4147;
/// `TrackPathZones`: the polygons that tag areas of the track (traffic patterns, tunnels, no-spawn...).
pub const TRACK_PATH_ZONES: u32 = 0x0003_414A;
/// `CarpWGrid` (`UWorld`): the world map tree: collision grid and road network.
pub const CARP_WGRID: u32 = 0x0003_B800;
/// `CarpWCollisionPack`: one section's static collision (`CARP` blob).
pub const CARP_WCOLLISION_PACK: u32 = 0x0003_B801;
/// `BoundsPack`: container of collision bounds (cars in `GlobalB.lzc`, props in the world file).
pub const BOUNDS_PACK: u32 = 0x8003_B900;
pub const BOUNDS: u32 = 0x0003_B901;
