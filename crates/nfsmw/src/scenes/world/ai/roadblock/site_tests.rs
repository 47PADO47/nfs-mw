use super::*;
use blackbox_roads::{RoadNetwork, RoadNode, RoadProfile, RoadSegment, Zone, zone};

fn road() -> TrafficWorld {
    let nodes = [-500.0, -30.0, 30.0, 500.0]
        .into_iter()
        .enumerate()
        .map(|(i, z)| RoadNode {
            position: Vec3::Z * z,
            profile: 0,
            segments: match i {
                0 => vec![0],
                3 => vec![2],
                _ => vec![i as u16 - 1, i as u16],
            },
        })
        .collect();
    let segments = (0..3)
        .map(|i| RoadSegment {
            nodes: [i, i + 1],
            length: [470.0, 60.0, 470.0][i as usize],
            road: None,
            flags: match i {
                1 => flags::DECISION | flags::INTERSECTION,
                _ => 0,
            },
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        })
        .collect();
    let zones = [-2.0, 2.0].map(|offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset }).to_vec();
    TrafficWorld::new(RoadNetwork { nodes, segments, profiles: vec![RoadProfile { middle: 1, zones }], roads: vec![] })
}

#[test]
fn roadblock_walk_crosses_a_junction_before_checking_its_destination() {
    let site = road().roadblock_site(Vec3::Z * -150.0, Vec3::Z).expect("valid road beyond the junction");
    assert!((site.centre.z - 100.0).abs() < 1.0, "{:?}", site.centre);
    assert!(site.forward.dot(Vec3::Z) > 0.99);
}

#[test]
fn roadblock_walk_starts_at_the_target_inside_a_junction() {
    let site = road().roadblock_site(Vec3::ZERO, Vec3::Z).expect("target's decision segment is a valid origin");
    assert!((site.centre.z - 250.0).abs() < 1.0, "{:?}", site.centre);
}

#[test]
fn roadblock_walk_still_rejects_a_destination_inside_a_junction() {
    let result = road().roadblock_site(Vec3::Z * -260.0, Vec3::Z);
    assert!(result.is_err_and(|reason| reason.contains("junction")));
}
