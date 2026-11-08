//! The pictures of the 2012 game the recipes use: the low 32 bits of the id of each texture in `HAWAII/GLOBALRESOURCES.BNDL`, with
//! what it is and its size. Found by reading the bundle's materials (`docs/formats/chameleon-world.md`, "Road, ground and sign
//! textures"): which sampler of which shader takes the texture says what it is.

/// The asphalt: the `ColourMap` of a driveable surface, 1024 x 1024 BC1. Grey, a little brown, with chips and wear.
pub const ASPHALT: u32 = 0x0006_175B;
/// A detail normal map of the aggregate in the road (`DetailMap_Normal`), 512 x 512 BC3.
pub const ASPHALT_NORMAL: u32 = 0x0012_86AF;
/// Concrete slabs: the `ColourMap` of the paved road, 2048 x 4096 BC1. A grid of slabs with cracks and moss.
pub const SLAB: u32 = 0x0004_2129;
/// The normal map of the slabs, 1024 x 2048 BC3.
pub const SLAB_NORMAL: u32 = 0x0004_2128;
/// Grass, `Tiling` layer of the ground, 2048 x 2048 BC1.
pub const GRASS: u32 = 0x000A_97AB;
/// The normal map of the grass, 1024 x 1024 BC3.
pub const GRASS_NORMAL: u32 = 0x000A_97AC;
/// A white road line, `Line_Diffuse`, 128 x 512 BC1: worn paint on black; the line is columns 10 to 113.
pub const WHITE_LINE: u32 = 0x0006_F394;
/// A yellow road line, the same texture in yellow: 128 x 512 BC1, columns 10 to 112.
pub const YELLOW_LINE: u32 = 0x0006_F393;
/// A concrete wall panel with a seam, 1024 x 512 BC1.
pub const WALL: u32 = 0x000D_4974;
/// The STOP sign: a 256 x 512 BC1 picture of the octagon (x 4 to 174, y 7 to 177), its back and the pole.
pub const STOP: u32 = 0x0006_D4E6;
/// The speed limit signs "20" and "50", 256 x 256 BC1, the faces turned a quarter turn: the 50 is the lower one.
pub const SPEED_SIGNS: u32 = 0x0008_C366;
/// The round "Fairhaven Drain" cover, 512 x 512 BC1 on black; its normal map (512 x 512 BC1).
pub const MANHOLE: u32 = 0x0006_D22D;
pub const MANHOLE_NORMAL: u32 = 0x0006_D22E;
/// The slotted storm grate, 256 x 256 BC1; its normal map (256 x 256 BC1).
pub const GRATE: u32 = 0x0006_D245;
pub const GRATE_NORMAL: u32 = 0x0006_D247;

/// Every texture the recipes read, for the tests.
#[cfg(test)]
pub const ALL: [u32; 15] = [
    ASPHALT,
    ASPHALT_NORMAL,
    SLAB,
    SLAB_NORMAL,
    GRASS,
    GRASS_NORMAL,
    WHITE_LINE,
    YELLOW_LINE,
    WALL,
    STOP,
    SPEED_SIGNS,
    MANHOLE,
    MANHOLE_NORMAL,
    GRATE,
    GRATE_NORMAL,
];
