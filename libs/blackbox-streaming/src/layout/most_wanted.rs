//! NFS: Most Wanted (PC). Field names from the decomp's `TrackStreamingSection`
//! and `VisibleSection.hpp`; every record of the v1.3 install checked
//! (`docs/formats/maps.md`).

use super::{BoundaryLayout, DrivableLayout, InfoLayout, LoadingLayout, SectionLayout, VisibleLayout};

/// 0x5C-byte records, verified on all 720 sections.
pub const MOST_WANTED: SectionLayout = SectionLayout {
    used_by: "NFS: Most Wanted (PC)",
    len: 0x5C,
    name: 0x00,
    number: 0x08,
    file_type: 0x10,
    file_offset: 0x14,
    size: 0x18,
    compressed_size: 0x1C,
    perm_size: 0x20,
    priority: 0x24,
    center: 0x28,
    radius: 0x30,
    checksum: 0x34,
};

/// Verified on all 515 boundaries, 435 drivable sections and 39 loading sections.
/// Every record starts with an 8-byte list node.
pub const MOST_WANTED_VISIBLE: VisibleLayout = VisibleLayout {
    used_by: "NFS: Most Wanted (PC)",
    info: InfoLayout { lod_offset: 0x00, region_count: 0x04, region_list: 0x08, region_max: 400 },
    boundary: BoundaryLayout {
        section: 0x08,
        num_points: 0x0A,
        panorama: 0x0B,
        bbox_min: 0x0C,
        bbox_max: 0x14,
        centre: 0x1C,
        points: 0x24,
        point_len: 8,
    },
    drivable: DrivableLayout { section: 0x0C, max_visible: 0x0F, num_visible: 0x10, list: 0x12, trailing: 2 },
    loading: LoadingLayout {
        len: 0x4C,
        name: 0x08,
        name_len: 15,
        default_flag: 0x17,
        num_drivable: 0x18,
        drivable: 0x1A,
        drivable_max: 16,
        num_extra: 0x3A,
        extra: 0x3C,
        extra_max: 8,
    },
};
