//! Real actor feeds keep independent tractor/trailer/cop roots and part identities.

use super::*;
use crate::scenes::world::ai::{
    TrafficWorld,
    car::{AiCar, Role, Start},
    traffic::Cruise,
};
use blackbox_roads::{LaneType, NavKind, RoadNav, SegmentFilter, SegmentIndex};

#[test]
#[ignore = "requires installed traffic, cop, trailer and highway collision"]
fn installed_road_vehicle_feed_covers_traffic_cops_and_independent_trailers() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let network = index.road_network.as_ref().unwrap();
    let (mut spawn, _, _) = highway(network);
    let collision = collision(&dir, &index, spawn);
    spawn.position.z =
        road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap().height;
    let physics = PhysicsData::load(&dir).unwrap();
    let ground = crate::scenes::world::drive::WorldGround { collision: &collision, surfaces: &physics.surfaces };
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
    let mut semi = model(&dir, &physics, "semilog", &mut renderer);
    semi.trailer = Some(Box::new(model(&dir, &physics, "trailerlog", &mut renderer)));
    let cop = model(&dir, &physics, "copmidsize", &mut renderer);
    let mut traffic = TrafficWorld::new(network.clone());
    let segments = SegmentIndex::build(network);
    let heading = Vec3::from(space::to_physics(Vec3::new(spawn.heading.cos(), spawn.heading.sin(), 0.0)));
    for (model, patrol) in [(&semi, false), (&cop, true)] {
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        assert!(nav.init_at_point(network, &segments, Vec3::from(space::to_physics(spawn.position)), heading, false));
        let start = Start { stagger: 0, speed: 0.0, cruise: Cruise::default(), patrol };
        traffic.cars.push(AiCar::place(model, Role::Traffic, nav, spawn, start, &ground).unwrap());
    }
    traffic.alpha = 0.37;
    let mut normal = Vec::new();
    traffic.instances(&mut normal);
    let mut wet = Vec::new();
    traffic.road_reflection_instances(&mut wet);
    assert!(!normal.is_empty());
    assert_eq!(wet.len(), normal.len());
    let by_key: std::collections::HashMap<_, _> = wet.iter().map(|part| (part.key, part)).collect();
    assert_eq!(by_key.len(), wet.len(), "actors sharing rigs must retain distinct instance keys");
    let mut expected = 0;
    let mut trailers = 0;
    for car in &traffic.cars {
        let root = car.pose(traffic.alpha).transform();
        let mut parts = Vec::new();
        car.instances(traffic.alpha, &mut parts);
        let mut trailer_parts = Vec::new();
        if let Some(trailer) = car.trailer() {
            trailer.instances(traffic.alpha, &mut trailer_parts);
            assert!(!trailer_parts.is_empty());
            trailers += 1;
        }
        for part in parts {
            let source_root = match trailer_parts.iter().any(|p| p.key == part.key) {
                true => car.trailer().unwrap().pose(traffic.alpha).transform(),
                false => root,
            };
            let mut reflected = source_root;
            reflected.z_axis = -reflected.z_axis;
            let captured = by_key[&part.key];
            assert_eq!(captured.mesh, part.mesh);
            assert!(captured.transform.abs_diff_eq(reflected * (source_root.inverse() * part.transform), 0.001));
            expected += 1;
        }
    }
    assert_eq!(trailers, 1, "the real articulated trailer must enter the capture");
    assert_eq!(expected, wet.len());
}
