//! Chunk ids used by this project.
//!
//! Names follow the community/decomp names in `tools/bchunk_names.py` (the full
//! table, sourced from the CC0 `dbalatoni13/nfsmw` decompilation's chunk list).
//! Only ids the Rust code reads are listed here.

// Geometry: docs/formats/models.md
pub const GEOMETRY_PACK: u32 = 0x8013_4000;
pub const MESH_CONTAINER_INFO: u32 = 0x8013_4001;
pub const MESH_CONTAINER_HEADER: u32 = 0x0013_4002;
pub const SOLID_PACK: u32 = 0x8013_4010;
pub const SOLID_INFO: u32 = 0x0013_4011;
pub const SOLID_TEXTURES: u32 = 0x0013_4012;
pub const SOLID_LIGHT_MATERIALS: u32 = 0x0013_4013;
pub const MESH_INFO_CONTAINER: u32 = 0x8013_4100;
pub const MESH_INFO_HEADER: u32 = 0x0013_4900;
pub const MESH_VERTEX_BUFFER: u32 = 0x0013_4B01;
pub const MESH_SHADING_GROUPS: u32 = 0x0013_4B02;
pub const MESH_INDICES: u32 = 0x0013_4B03;

// Textures: docs/formats/textures.md
pub const TEXTURE_PACK: u32 = 0xB330_0000;
pub const TEXTURE_PACK_INFO: u32 = 0xB331_0000;
pub const TEXTURE_PACK_INFO_HEADER: u32 = 0x3331_0001;
pub const TEXTURE_PACK_INFO_KEYS: u32 = 0x3331_0002;
pub const TEXTURE_PACK_INFO_ENTRIES: u32 = 0x3331_0003;
pub const TEXTURE_PACK_INFO_TEXTURES: u32 = 0x3331_0004;
pub const TEXTURE_PACK_INFO_COMPS: u32 = 0x3331_0005;
pub const TEXTURE_PACK_DATA: u32 = 0xB332_0000;
pub const TEXTURE_PACK_DATA_HEADER: u32 = 0x3332_0001;
pub const TEXTURE_PACK_DATA_ARRAY: u32 = 0x3332_0002;
