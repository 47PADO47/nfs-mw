//! Road nodes (`RNnd`).

use glam::Vec3;

use crate::Result;
use crate::bytes::Reader;
use crate::error::malformed;

pub(crate) const NODE_LEN: usize = 32;
const MAX_SEGMENTS: usize = 7;

#[derive(Debug, Clone, PartialEq)]
pub struct RoadNode {
    pub position: Vec3,
    /// Index into the profile table.
    pub profile: u16,
    /// Segments attached to the node (1…7).
    pub segments: Vec<u16>,
}

impl RoadNode {
    pub(crate) fn parse(r: &Reader<'_>, at: usize, index: usize) -> Result<Self> {
        let own = r.i16(at + 0x0C)?;
        if usize::try_from(own) != Ok(index) {
            return Err(malformed("road node", format!("node {index} stores index {own}")));
        }
        let count = usize::from(r.u8(at + 0x10)?);
        if count > MAX_SEGMENTS {
            return Err(malformed("road node", format!("node {index} has {count} segments")));
        }
        let segments = (0..count).map(|i| r.u16(at + 0x12 + i * 2)).collect::<Result<_>>()?;
        Ok(Self { position: r.vec3(at)?, profile: r.u16(at + 0x0E)?, segments })
    }
}
