//! Hand-built networks for tests.

use glam::Vec3;

use crate::{Road, RoadNetwork, RoadNode, RoadProfile, RoadSegment, Zone, zone};

pub fn lane(kind: u8, offset: f32) -> Zone {
    Zone { kind, width: 4.0, offset }
}

/// Shoulder, two lanes left of the middle, two lanes right of it, shoulder: the stored-direction lanes
/// are the right two.
pub fn two_way_profile() -> RoadProfile {
    RoadProfile {
        middle: 3,
        zones: vec![
            lane(zone::SHOULDER, 10.0),
            lane(zone::TRAFFIC, 6.0),
            lane(zone::TRAFFIC, 2.0),
            lane(zone::TRAFFIC, 2.0),
            lane(zone::TRAFFIC, 6.0),
            lane(zone::SHOULDER, 10.0),
        ],
    }
}

pub fn segment(nodes: [u16; 2], length: f32, flags: u16) -> RoadSegment {
    RoadSegment { nodes, length, road: Some(0), flags, start_handle: Vec3::ZERO, end_handle: Vec3::ZERO }
}

/// A straight 100 m road along +z from the origin, one segment, `two_way_profile` at both ends.
pub fn straight_road() -> RoadNetwork {
    let node = |z: f32, segments: Vec<u16>, index: usize| {
        let _ = index;
        RoadNode { position: Vec3::new(0.0, 0.0, z), profile: 0, segments }
    };
    RoadNetwork {
        nodes: vec![node(0.0, vec![0], 0), node(100.0, vec![0], 1)],
        segments: vec![segment([0, 1], 100.0, 0)],
        profiles: vec![two_way_profile()],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    }
}
