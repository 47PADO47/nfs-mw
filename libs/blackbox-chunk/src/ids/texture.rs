//! Textures: `docs/formats/textures.md`.

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

/// `TextureAnimPack`: animated textures.
pub const TEXTURE_ANIM_PACK: u32 = 0xB030_0100;
pub const TEXTURE_ANIM_PACK_HEADER: u32 = 0x3030_0101;
/// 0x34-byte `TextureAnim` records.
pub const TEXTURE_ANIM_PACK_ANIMS: u32 = 0x3030_0102;
/// 16-byte frame entries (texture name hash + runtime pointers).
pub const TEXTURE_ANIM_PACK_FRAMES: u32 = 0x3030_0103;
