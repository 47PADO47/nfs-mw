use super::*;
use game_install::GameDir;
use nfsmw_data::car::{LoadOptions, physics::PhysicsData};

#[test]
#[ignore = "needs installed cop geometry, physics and pursuit tables"]
fn installed_free_roam_pursuit_uses_the_real_patrol_car_and_heat_rows() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let rows = nfsmw_data::pursuit::heat_rows(data.database());
    assert_eq!(rows.len(), 10);
    assert_eq!(rows[0].patrol_cars, 1);
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    let mut model = cop_model(&mut renderer, "copmidsize", player());
    model.model.physics = data.car("copmidsize").unwrap();
    let car =
        nfsmw_data::car::load(&dir, "COPMIDSIZE", &LoadOptions { lod: 'A', all_parts: false, preset: None }).unwrap();
    assert!(!car.placements.is_empty());
    model.model.rig =
        Rc::new(CarRig::upload(&mut renderer, keys::next_car(), car, CarShading::default(), sun::to_sun()));
    model.car.ai = nfsmw_data::pursuit::ai_vehicle(data.database(), "copmidsize");
    let mut world = TrafficWorld::new(cop_road());
    world.load_data(data.database(), None);
    let mut nav = RoadNav::traffic();
    assert!(nav.init_at_point(&world.network, &world.index, Vec3::new(2.0, 0.0, 20.0), Vec3::Z, false));
    let car = AiCar::place(
        &model.model,
        Role::Traffic,
        nav,
        Spawn { position: space::to_render([2.0, 0.0, 20.0]), heading: 0.0 },
        Start { speed: 10.0, stagger: 0, cruise: Cruise::default(), patrol: true },
        &WorldGround { collision: &floor(), surfaces: &SurfaceTable::default() },
    )
    .expect("installed patrol stands");
    world.cars.push(car);
    world.add_cop_model(model);
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, rows, player());
    assert_eq!(cop_count(&world), 1);
    assert!(world.cars[0].emergency_lights());
    assert_eq!(world.pursuit_feed().unwrap().heat, 1);
    let mut blips = Vec::new();
    world.map_blips(&mut blips);
    assert_eq!(blips.len(), 1);
    assert!(blips[0].active);
    world.stop_pursuit();
    assert!(world.cars.is_empty());
}
