//! One streaming section.

#[derive(Debug, Clone, PartialEq)]
pub struct StreamingSection {
    /// E.g. `A41`, `X0`.
    pub name: String,
    /// Letter index × 100 + number (`A41` = 141).
    pub number: i16,
    pub file_type: i32,
    /// Byte range in the stream file.
    pub file_offset: u32,
    pub size: u32,
    pub compressed_size: u32,
    pub perm_size: u32,
    pub priority: i32,
    /// Map position (x, y); (0, 0) for non-spatial sections.
    pub center: [f32; 2],
    pub radius: f32,
    pub checksum: u32,
}

impl StreamingSection {
    /// Whether the section is a map tile (has a position) rather than a shared
    /// set loaded alongside the tiles.
    pub fn is_spatial(&self) -> bool {
        self.radius > 0.0
    }

    /// Distance from a map point to the section's bounding circle (0 inside it).
    pub fn distance_to(&self, x: f32, y: f32) -> f32 {
        let (dx, dy) = (x - self.center[0], y - self.center[1]);
        ((dx * dx + dy * dy).sqrt() - self.radius).max(0.0)
    }

    /// Byte range in the stream file.
    pub fn range(&self) -> std::ops::Range<u64> {
        u64::from(self.file_offset)..u64::from(self.file_offset) + u64::from(self.size)
    }
}
