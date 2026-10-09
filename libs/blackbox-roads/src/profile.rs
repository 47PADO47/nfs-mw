//! Road profiles: the lane layout (zones) shared by every segment that ends at a node.
//! Spec: `docs/formats/road-network.md` (`RNpf`).

use crate::Result;
use crate::bytes::Reader;
use crate::error::malformed;

pub(crate) const PROFILE_LEN: usize = 64;
const MAX_ZONES: usize = 15;
/// Zone distances are stored as signed 14-bit values of `100 / 8191` metres.
const ZONE_SCALE: f32 = 100.0 / 8191.0;

/// Zone types by their stored number (the order of the engine's `kRoadProfile*` constants).
pub mod zone {
    pub const TRAFFIC: u8 = 1;
    pub const SIDEWALK: u8 = 2;
    pub const SHOULDER: u8 = 3;
    pub const MEDIAN: u8 = 4;
    pub const CURB_MEDIAN: u8 = 5;
    pub const GRASS_MEDIAN: u8 = 6;
    pub const BARRIER: u8 = 7;
    pub const TRAIN: u8 = 8;
    pub const PARKING: u8 = 9;
    pub const ROAD: u8 = 10;
    pub const CENTER: u8 = 11;
    pub const DRIVABLE: u8 = 12;
    pub const UNDRIVABLE: u8 = 13;
}

/// One lane (or strip) of a profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    pub kind: u8,
    /// Width in metres.
    pub width: f32,
    /// Distance of the zone's centre from the road centre line, always positive; the side comes from
    /// the zone index (see [`RoadProfile::signed_offset`]).
    pub offset: f32,
}

impl Zone {
    fn unpack(value: u32) -> Self {
        let signed14 = |bits: u32| (((bits & 0x3FFF) << 18) as i32 >> 18) as f32;
        Self {
            kind: (value & 0xF) as u8,
            width: signed14(value >> 4) * ZONE_SCALE,
            offset: signed14(value >> 18) * ZONE_SCALE,
        }
    }

    /// Whether a car whose lane type allows the zone types in `mask` (bit `n` is type `n`) may use it.
    pub fn in_mask(&self, mask: u16) -> bool {
        mask & (1 << self.kind) != 0
    }
}

/// The zones of a road cross section, left to right in the stored segment direction.
#[derive(Debug, Clone, PartialEq)]
pub struct RoadProfile {
    /// Index of the first zone right of the centre line.
    pub middle: u8,
    pub zones: Vec<Zone>,
}

impl RoadProfile {
    pub(crate) fn parse(r: &Reader<'_>, at: usize) -> Result<Self> {
        let count = usize::from(r.u8(at)?);
        let middle = r.u8(at + 1)?;
        if count > MAX_ZONES || usize::from(middle) > count {
            return Err(malformed("road profile", format!("{count} zones, middle {middle}")));
        }
        let zones = (0..count).map(|i| Ok(Zone::unpack(r.u32(at + 4 + i * 4)?))).collect::<Result<_>>()?;
        Ok(Self { middle, zones })
    }

    /// Zones that run in the stored direction (right of the middle).
    pub fn forward_lanes(&self) -> usize {
        self.zones.len() - usize::from(self.middle)
    }

    /// Zones that run against the stored direction (left of the middle).
    pub fn backward_lanes(&self) -> usize {
        usize::from(self.middle)
    }

    /// The offset of zone `index` from the centre line, positive to the right in the stored direction.
    pub fn signed_offset(&self, index: usize) -> f32 {
        let zone = &self.zones[index];
        match index < usize::from(self.middle) {
            true => -zone.offset,
            false => zone.offset,
        }
    }

    /// The profile read the other way round: zones reversed, the middle mirrored. Segments flag the
    /// end nodes whose profile is stored for the opposite orientation.
    pub fn inverted(&self) -> Self {
        let mut zones = self.zones.clone();
        zones.reverse();
        Self { middle: (self.zones.len() - usize::from(self.middle)) as u8, zones }
    }

    /// Indices of the zones of `kind` on one side, ordered from the centre line outwards.
    pub fn lanes_of(&self, kind: u8, right: bool) -> Vec<usize> {
        let middle = usize::from(self.middle);
        let mut found: Vec<usize> =
            (0..self.zones.len()).filter(|&i| self.zones[i].kind == kind && (i >= middle) == right).collect();
        if !right {
            found.reverse();
        }
        found
    }
}
