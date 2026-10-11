//! Road segments (`RNsg`).

use glam::Vec3;

use crate::Result;
use crate::bytes::Reader;
use crate::error::malformed;

pub(crate) const SEGMENT_LEN: usize = 22;

/// Segment flag bits (`fFlags`). The bits marked runtime are zero in the file; race setup and barrier
/// changes set them.
pub mod flags {
    /// Connector inside a junction; always together with [`INTERSECTION`].
    pub const DECISION: u16 = 1 << 0;
    pub const NO_TRAFFIC: u16 = 1 << 1;
    /// Runtime: race direction.
    pub const RACE_ROUTE_FORWARD: u16 = 1 << 2;
    pub const INTERSECTION: u16 = 1 << 3;
    /// Plain road end that enters a junction.
    pub const ENTRANCE: u16 = 1 << 4;
    /// Flips the no-traffic test for cops.
    pub const COPS_XOR_TRAFFIC: u16 = 1 << 5;
    /// One-way in the stored direction (start to end).
    pub const ONE_WAY: u16 = 1 << 6;
    /// Runtime: set by race setup.
    pub const SHORTCUT: u16 = 1 << 7;
    /// Bézier segment.
    pub const CURVED: u16 = 1 << 8;
    pub const END_INVERTED: u16 = 1 << 9;
    pub const START_INVERTED: u16 = 1 << 10;
    pub const CHOPPER_STAY_LOW: u16 = 1 << 11;
    /// Runtime: computed from the barriers.
    pub const CROSSES_BARRIER: u16 = 1 << 12;
    /// Runtime: computed from the barriers.
    pub const CROSSES_DRIVE_THROUGH_BARRIER: u16 = 1 << 13;
    /// No reader in the original; meaning unknown.
    pub const LANE_MAP: u16 = 1 << 14;
    /// Runtime: on the current race route.
    pub const IN_RACE: u16 = 1 << 15;
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoadSegment {
    /// Start and end node of the stored direction.
    pub nodes: [u16; 2],
    /// Arc length in metres.
    pub length: f32,
    /// Index into the road table, if the segment belongs to a road.
    pub road: Option<u16>,
    pub flags: u16,
    /// Bézier handle at the start node, pointing along the segment (metres).
    pub start_handle: Vec3,
    /// Bézier handle at the end node, pointing back into the segment (metres).
    pub end_handle: Vec3,
}

impl RoadSegment {
    pub(crate) fn parse(r: &Reader<'_>, at: usize, index: usize) -> Result<Self> {
        let own = r.i16(at + 0x08)?;
        if usize::try_from(own) != Ok(index) {
            return Err(malformed("road segment", format!("segment {index} stores index {own}")));
        }
        let direction = |o: usize| -> Result<Vec3> {
            Ok(Vec3::new(f32::from(r.i8(o)?), f32::from(r.i8(o + 1)?), f32::from(r.i8(o + 2)?)) / 127.0)
        };
        let handle = |length_at: usize, dir_at: usize| -> Result<Vec3> {
            Ok(direction(dir_at)? * (f32::from(r.u16(length_at)?) * 500.0 / 65535.0))
        };
        Ok(Self {
            nodes: [r.u16(at)?, r.u16(at + 2)?],
            length: f32::from(r.u16(at + 4)?) * 1000.0 / 65535.0,
            road: u16::try_from(r.i16(at + 6)?).ok(),
            flags: r.u16(at + 0x0A)?,
            end_handle: handle(at + 0x0C, at + 0x10)?,
            start_handle: handle(at + 0x0E, at + 0x13)?,
        })
    }

    pub fn has(&self, flag: u16) -> bool {
        self.flags & flag != 0
    }

    pub fn is_decision(&self) -> bool {
        self.has(flags::DECISION)
    }

    pub fn is_curved(&self) -> bool {
        self.has(flags::CURVED)
    }

    pub fn is_one_way(&self) -> bool {
        self.has(flags::ONE_WAY)
    }

    /// The node at the other end of the segment than `node`, if `node` is one of its ends.
    pub fn other_node(&self, node: u16) -> Option<u16> {
        match node {
            n if n == self.nodes[0] => Some(self.nodes[1]),
            n if n == self.nodes[1] => Some(self.nodes[0]),
            _ => None,
        }
    }
}
