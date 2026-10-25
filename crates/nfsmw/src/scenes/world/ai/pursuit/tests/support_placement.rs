use super::*;

fn pair_world(heat: u32) -> TrafficWorld {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    for name in ["copsuvl", "copsuv"] {
        world.add_cop_model(cop_model(&mut free_roam::renderer(), name, player()));
    }
    world.set_career_bin(Some(16));
    world.police.set_forced_pursuit(true);
    world.start_pursuit(heat, vec![wave(); 5], player());
    assert_eq!(world.police.level(), Some(heat));
    world
}

fn deploy_pair(world: &mut TrafficWorld, focus: Focus) {
    assert!(world.ask_support(support(Strategy::Ram)));
    world.update_support(FRAME, focus, &floor(), &SurfaceTable::default());
    world.update_support(3.1, focus, &floor(), &SurfaceTable::default());
    assert!(support_order(world).unwrap().is_active());
    assert_eq!(support_count(world), 2, "one request must deploy its complete pair");
}

#[test]
fn light_and_heavy_ram_pairs_keep_the_authored_ten_metre_spacing() {
    for heat in [3, 4, 5] {
        let mut world = pair_world(heat);
        deploy_pair(&mut world, support_focus(-150.0));
        let cars: Vec<_> = world.cars.iter().filter(|car| car.support_serial().is_some()).collect();
        let expected = match heat {
            3 => "copsuvl",
            _ => "copsuv",
        };
        assert!(cars.iter().all(|car| car.name.eq_ignore_ascii_case(expected)), "heat {heat}");
        let gap = cars[0].physics_position().distance(cars[1].physics_position());
        assert!((gap - 10.0).abs() < 0.1, "heat {heat}: pair gap={gap}");
        assert!(gap >= cars[0].bounding_radius() + cars[1].bounding_radius() + 2.0);
        assert!(cars.iter().all(|car| car.physics_forward().dot(-Vec3::Z) > 0.99));
    }
}

#[test]
fn ram_pair_walks_around_a_bend_and_faces_back_along_the_selected_road() {
    let mut world = pair_world(3);
    let prototype = world.network.segments[0].clone();
    world.network.nodes = [
        (Vec3::new(0.0, 0.0, -100.0), vec![0]),
        (Vec3::new(0.0, 0.0, 200.0), vec![0, 1]),
        (Vec3::new(500.0, 0.0, 200.0), vec![1]),
    ]
    .into_iter()
    .map(|(position, segments)| blackbox_roads::RoadNode { position, profile: 0, segments })
    .collect();
    world.network.segments = [[0, 1], [1, 2]]
        .into_iter()
        .enumerate()
        .map(|(i, nodes)| blackbox_roads::RoadSegment { nodes, length: [300.0, 500.0][i], ..prototype.clone() })
        .collect();
    world.index = blackbox_roads::SegmentIndex::build(&world.network);
    deploy_pair(&mut world, support_focus(0.0));
    let mut cars = world.cars.iter().filter(|car| car.support_serial().is_some());
    for x in [140.0, 130.0] {
        let car = cars.next().unwrap();
        let at = car.physics_position();
        assert!((at.x - x).abs() < 1.0 && (at.z - 200.0).abs() < 2.0, "{at:?}");
        assert!(car.physics_forward().dot(-Vec3::X) > 0.99);
    }
}

#[test]
#[ignore = "needs installed SUV/player physics and support heat rows selected by NFSMW_GAME_DIR"]
fn installed_ram_pairs_approach_and_enter_against_a_fast_target() {
    use blackbox_pursuit::support::ram::DRIVE_SPEED;
    use game_install::GameDir;
    use nfsmw_data::car::physics::PhysicsData;

    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let player_physics = data.car("bmwm3gtr").unwrap();
    let performance = nfsmw_data::car::physics::measured::ai_performance_of_spec(
        &player_physics.spec,
        player_physics.tire_integration,
    );
    let collision = super::installed_spawn_drive::corridor();
    let initial = Body {
        position: Vec3::Y * 0.7,
        velocity: Vec3::Z * 60.0,
        forward: Vec2::Y,
        half_width: player_physics.spec.dimension.x,
        half_length: player_physics.spec.dimension.z,
    };
    let focus = |body: Body| Focus {
        position: [body.position.z, -body.position.x],
        heading: 0.0,
        speed: 60.0,
        body: Some(body),
    };
    for heat in [3, 4, 5] {
        let mut world = TrafficWorld::new(straight_road(3000.0));
        world.load_data(data.database(), None);
        world.configure(0, 0);
        for name in ["copsuvl", "copsuv"] {
            let mut model = cop_model(&mut free_roam::renderer(), name, performance);
            model.model.physics = data.car(name).unwrap();
            model.car.performance = nfsmw_data::car::physics::measured::ai_performance_of_spec(
                &model.model.physics.spec,
                model.model.physics.tire_integration,
            );
            model.car.ai = nfsmw_data::pursuit::ai_vehicle(data.database(), name);
            world.add_cop_model(model);
        }
        world.set_career_bin(Some(16));
        world.police.set_forced_pursuit(true);
        world.start_pursuit(heat, nfsmw_data::pursuit::heat_rows(data.database()), performance);
        assert!(world.ask_support(support(Strategy::Ram)));
        world.update_support(FRAME, focus(initial), &collision, &data.surfaces);
        world.update_support(3.1, focus(initial), &collision, &data.surfaces);
        assert!(support_order(&world).unwrap().is_active(), "heat {heat}");
        assert_eq!(support_count(&world), 2);
        let gap = world.cars[0].physics_position().distance(world.cars[1].physics_position());
        assert!((gap - 10.0).abs() < 0.1, "installed heat {heat}: initial gap={gap}");
        let mut props = crate::scenes::world::props::PropWorld::default();
        for tick in 0..180 {
            let seconds = tick as f32 * crate::scenes::world::drive::STEP;
            let body = Body { position: initial.position + initial.velocity * seconds, ..initial };
            world.step(focus(body), &collision, &mut props, &data.surfaces);
            world.collide(None);
        }
        let pair: Vec<_> = world.cars.iter().filter(|car| car.support_serial().is_some()).collect();
        assert_eq!(pair.len(), 2, "both original support vehicles must remain");
        for car in pair {
            eprintln!(
                "SUV_PAIR heat={heat} model={} body={:?} request={:?}",
                car.name,
                car.body(),
                car.drive_request()
            );
            assert!(car.is_finite() && car.speed() > 5.0);
            assert!(car.physics_forward().dot(-Vec3::Z) > 0.5, "SUV must keep approaching");
            assert_eq!(car.drive_request().1, DRIVE_SPEED, "fast target must not veto ram entry");
        }
    }
}
