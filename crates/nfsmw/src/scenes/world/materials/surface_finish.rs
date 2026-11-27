//! Dry roughness for identified Rockport texture families.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Finish {
    Asphalt,
    Concrete,
    Unknown,
}

impl Finish {
    pub(super) fn for_diffuse(hash: u32) -> Self {
        if ASPHALT.contains(&hash) {
            return Self::Asphalt;
        }
        if CONCRETE_SIDEWALK.contains(&hash) {
            return Self::Concrete;
        }
        Self::Unknown
    }

    pub(super) fn roughness(self) -> f32 {
        match self {
            Self::Asphalt => 0.86,
            Self::Concrete => 0.68,
            Self::Unknown => 0.78,
        }
    }
}

// Diffuse identifiers only. See docs/formats/chameleon-road-textures.md and the
// stock texture identities recorded by the road replacement table.
const ASPHALT: &[u32] = &[
    0x06ee3a6d, 0xd5b76c7d, 0x10f7a61e, 0x422e740e, 0xb767f1ee, 0xdb34b2a4, 0x282eef65, 0xe85b4f4f, 0xdbd32c25,
    0xdbd78f2d, 0x28cd68e6, 0x28d1cbee, 0x491d7b26, 0x4921de2e, 0x9617b7e7, 0x961c1aef, 0xa0405c3d, 0xed3a98fe,
    0x25627894, 0xbe10f98a, 0x74c18215, 0xaa52ce6b,
];
const CONCRETE_SIDEWALK: &[u32] = &[0x916c5ac1, 0x93c18362, 0xb7bc8b97, 0xb7bc8b98];
