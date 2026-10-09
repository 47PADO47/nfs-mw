//! The track path zones: polygons on the map that tag areas (traffic patterns, tunnels, no-spawn
//! areas...). Spec: `docs/formats/road-network.md` ("Track path zones").
//!
//! Zones live in the track's 2D frame, not in physics space: see [`to_zone_space`].

use blackbox_chunk::ids;
use glam::{Vec2, Vec3};

use crate::Result;
use crate::bytes::Reader;
use crate::error::malformed;

/// Fixed part of a zone record; the polygon points follow.
const HEADER_LEN: usize = 0x44;
/// A polygon point is two f32.
const POINT_LEN: usize = 8;

/// A physics-space position as the point the zones are tested with: physics `x = -y2d`, `z = x2d`, so
/// the zone point is `(z, -x)`. Height is dropped.
pub fn to_zone_space(physics: Vec3) -> Vec2 {
    Vec2::new(physics.z, -physics.x)
}

/// The physics-space position of a zone-space point, at height `y`.
pub fn from_zone_space(point: Vec2, y: f32) -> Vec3 {
    Vec3::new(-point.y, y, point.x)
}

/// One zone: a typed polygon with four words of type-specific data.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackZone {
    /// What the zone tags (traffic pattern, tunnel...): ids are in `docs/formats/road-network.md`.
    pub kind: u32,
    pub position: Vec2,
    pub direction: Vec2,
    pub elevation: f32,
    /// Type-specific words; a traffic-pattern zone holds the hash of the pattern name in `data[0]`.
    pub data: [i32; 4],
    pub bbox_min: Vec2,
    pub bbox_max: Vec2,
    pub polygon: Vec<Vec2>,
}

impl TrackZone {
    /// Reads the record at the start of `bytes`; returns it with its size in bytes.
    fn parse(bytes: &[u8]) -> Result<(Self, usize)> {
        let r = Reader::new(bytes, "track zone");
        let vec2 = |o: usize| -> Result<Vec2> { Ok(Vec2::new(r.f32(o)?, r.f32(o + 4)?)) };
        let points = usize::try_from(r.i16(0x40)?).map_err(|_| malformed("track zone", "negative point count"))?;
        let size = usize::try_from(r.i16(0x42)?).map_err(|_| malformed("track zone", "negative record size"))?;
        if size != HEADER_LEN + POINT_LEN * points {
            return Err(malformed("track zone", format!("record size {size} for {points} points")));
        }
        let polygon = (0..points).map(|i| vec2(HEADER_LEN + POINT_LEN * i)).collect::<Result<_>>()?;
        let zone = Self {
            kind: r.u32(0)?,
            position: vec2(0x04)?,
            direction: vec2(0x0C)?,
            elevation: r.f32(0x14)?,
            data: [r.u32(0x30)? as i32, r.u32(0x34)? as i32, r.u32(0x38)? as i32, r.u32(0x3C)? as i32],
            bbox_min: vec2(0x20)?,
            bbox_max: vec2(0x28)?,
            polygon,
        };
        Ok((zone, size))
    }

    /// Whether the zone-space `point` is inside the polygon (even-odd rule, bounding box tested first).
    pub fn contains(&self, point: Vec2) -> bool {
        if point.cmplt(self.bbox_min).any() || point.cmpgt(self.bbox_max).any() {
            return false;
        }
        let mut inside = false;
        let Some(mut previous) = self.polygon.last().copied() else { return false };
        for &current in &self.polygon {
            let crosses = (current.y > point.y) != (previous.y > point.y);
            if crosses {
                let x_at = current.x + (point.y - current.y) / (previous.y - current.y) * (previous.x - current.x);
                inside ^= point.x < x_at;
            }
            previous = current;
        }
        inside
    }
}

/// All zones of a track, in file order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackZones {
    pub zones: Vec<TrackZone>,
}

impl TrackZones {
    /// Parses the payload of a zones chunk: records back to back, each sized by its own field.
    pub fn parse(payload: &[u8]) -> Result<Self> {
        let mut zones = Vec::new();
        let mut at = 0;
        while at < payload.len() {
            let (zone, size) = TrackZone::parse(&payload[at..])?;
            zones.push(zone);
            at += size;
        }
        Ok(Self { zones })
    }

    /// The zones in `data` (the track's world metadata file), if it has any.
    pub fn read(data: &[u8]) -> Result<Option<Self>> {
        let Some(manager) = blackbox_chunk::find(data, ids::TRACK_PATH_MANAGER) else { return Ok(None) };
        let Some(chunk) = manager.find(ids::TRACK_PATH_ZONES) else { return Ok(None) };
        Self::parse(chunk.payload).map(Some)
    }

    /// The zones of `kind` that contain the zone-space `point`, in file order.
    pub fn contains_point(&self, kind: u32, point: Vec2) -> Vec<&TrackZone> {
        self.zones.iter().filter(|z| z.kind == kind && z.contains(point)).collect()
    }

    /// The first zone of `kind` (file order) that contains the zone-space `point`.
    pub fn first_of_kind_at(&self, kind: u32, point: Vec2) -> Option<&TrackZone> {
        self.zones.iter().find(|z| z.kind == kind && z.contains(point))
    }
}
