//! Private mounted-fixture test for an impact on the coupled concrete hauler.
use super::*;
use blackbox_collision::bounds_flags;

#[test]
#[ignore = "requires the concrete-release mod fixture in NFSMW_GAME_DIR"]
fn concrete_rear_impact_unloads_the_moving_rig_without_unhitching() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to the pipe fixture");
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let (mut spawn, _, _) = highway(index.road_network.as_ref().unwrap());
    let collision = collision(&dir, &index, spawn);
    spawn.position.z =
        road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.).unwrap().height;
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let tractor_model = model(&dir, &data, "semicmt", &mut renderer);
    let trailer_name = data.trailer_of("semicmt").unwrap();
    assert_eq!(trailer_name, "trailercmt");
    let trailer_model = model(&dir, &data, &trailer_name, &mut renderer);
    let mut truck = CarSim::for_rig(tractor_model.physics.clone(), &tractor_model.rig);
    truck.configure_traffic();
    assert!(truck.place_traffic_moving(&ground, spawn, 10.));
    let mut load = TrailerCar::place(&trailer_model, &truck, spawn, 10., &ground).unwrap();
    let mut props = PropWorld::default();
    let input = DriveInput { throttle: 0.15, ..DriveInput::default() };
    let mut gap = 0_f32;
    let concrete = data.fragment_dynamics(vlt_hash("heavyconcreteobject")).unwrap();
    let mut on_road = std::collections::BTreeSet::new();
    let mut last_tick = 0;
    for tick in 0..=390 {
        props.step(&ground, STEP);
        truck.step(&input, &ground, Some((&collision, &mut props)));
        load.step(&mut truck, Brakes::default(), &collision, &mut props, &data.surfaces);
        if tick == 30 {
            assert_eq!(props.fragment_count(), 0);
            let sim = &mut load.sim;
            let body = sim.body();
            let orientation = body.orientation;
            let forward = orientation * Vec3::Z;
            let prim = sim
                .geometry
                .as_ref()
                .unwrap()
                .body
                .primitives
                .iter()
                .filter(|p| p.collides(bounds_flags::PRIM_VS_OBJECTS))
                .min_by(|a, b| (a.centre.z - a.half.z).total_cmp(&(b.centre.z - b.half.z)))
                .unwrap();
            let rear = crate::scenes::world::object_geometry::primitive_at(prim, (body.position, orientation));
            let mut striker_physics = data.car("911turbo").unwrap();
            striker_physics.geometry = None;
            let mut car = CarSim::new(striker_physics, [0.2; 4]);
            let position = rear.centre - forward * (rear.half.z + car.half_dimensions().z + 0.5);
            car.body_mut().place(position, orientation);
            car.body_mut().linear_velocity = body.linear_velocity + forward * 48.;
            let mut hit = None;
            // Resolve on the first actual contact tick rather than assuming a particular
            // pair of boxes intersects after exactly one integration step.
            let air = FlatGround::new(-100.);
            for _ in 0..6 {
                car.step(&DriveInput::default(), &air, None);
                hit = car.collide_with(sim);
                if hit.is_some() {
                    break;
                }
            }
            assert!(
                hit.is_some(),
                "the striker must hit the coupled trailer: rear={rear:?}, car={:?}, trailer={:?}, forward={forward:?}, velocity={:?}",
                car.body().position,
                sim.body().position,
                car.body().linear_velocity
            );
        }
        load.update_events(&mut props, &data, truck.pose().position);
        load.resolve_articulation(&mut truck);
        load.sync_contacts();
        gap = gap.max(load.coupling.gap(&truck, &load.sim));
        assert!(load.is_hitched(), "hitch released at {tick}, gap={gap}, {:?} {:?}", truck.up(), load.sim.up());
        if tick < 30 {
            continue;
        }
        assert_eq!(props.fragment_count(), 8, "the load and straps must stay separate at tick {tick}");
        let rot = load.sim.body().rotation().transpose();
        for (id, body, _) in props.fragment_bodies() {
            if body.mass() != concrete.mass {
                continue;
            }
            let local = rot * (body.position - load.sim.body().position);
            let pos = space::to_render(body.position.to_array());
            let height = road::probe(&collision, pos.x, pos.y, pos.z + 2.).map(|s| pos.z - s.height);
            if (local.z.abs() > 8.2 || local.x.abs() > 2.8)
                && height.is_some_and(|h| (0.95..1.58).contains(&h))
                && on_road.insert(id)
            {
                eprintln!(
                    "pipe {id} clears deck and reaches road at tick {tick}; ground clearance {height:?} m, relative {local:?}"
                );
            }
            if tick == 390 {
                eprintln!("final pipe {id} relative {local:?}, above road {height:?} m");
            }
        }
        last_tick = tick;
        if tick >= 180 && on_road.len() == 4 {
            break;
        }
    }
    assert!(gap < 0.1);
    assert_eq!(on_road.len(), 4, "all four concrete pipes should reach the road outside the deck");
    eprintln!(
        "SEMICMT struck by a 911 Turbo at 48 m/s relative speed on an authored highway. Eight pieces separated. All four pipes cleared the moving deck and reached road level. Hitch remained engaged through {} post-impact ticks, maximum post-solve gap {gap:.5} m.",
        last_tick - 30
    );
}
