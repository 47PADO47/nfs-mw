//! NFS: Most Wanted (PC): 0x5C-byte records, verified on all 720 sections of the
//! v1.3 install; field names from the decomp's `TrackStreamingSection`.

use super::SectionLayout;

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
