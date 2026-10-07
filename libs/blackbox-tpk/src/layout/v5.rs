//! TPK version 5: NFS: Most Wanted (PC). Verified on the PC v1.3 install; field
//! names from the decomp's `TextureInfo` (see `docs/formats/textures.md`).

use super::{InfoLayout, PackLayout};

pub const V5: PackLayout = PackLayout {
    version: 5,
    used_by: "NFS: Most Wanted (PC)",
    info: InfoLayout {
        len: 0x7C,
        name: 0x0C..0x24,
        name_hash: 0x24,
        image_placement: 0x30,
        palette_placement: 0x34,
        image_size: 0x38,
        palette_size: 0x3C,
        width: 0x44,
        height: 0x46,
        compression_type: 0x4A,
        mip_levels: 0x4E,
        alpha_sorting: 0x54,
        alpha_usage: 0x55,
        alpha_blend: 0x56,
    },
    plat_len: 0x20,
    plat_format: 0x14,
    entry_len: 24,
    data_align: 0x80,
};
