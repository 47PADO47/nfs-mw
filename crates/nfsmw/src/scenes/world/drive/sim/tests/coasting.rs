use super::*;

#[test]
fn coasting_game_selector_excludes_player_live_police_and_normal_traffic() {
    let ground = FlatGround::new(0.0);
    for (player, cop, destroyed) in
        [(true, true, true), (false, true, false), (false, false, true), (false, true, true)]
    {
        let mut car = sim();
        if !player {
            car.configure_ai();
        }
        car.collision_role.cop = cop;
        assert!(car.place(&ground, Spawn { position: Vec3::ZERO, heading: 0.0 }));
        for _ in 0..120 {
            car.step(&DriveInput::default(), &ground, None);
        }
        car.set_destroyed_non_player(destroyed);
        // A new patch velocity with old wheel speed distinguishes no-slip from racer history.
        car.body_mut().linear_velocity = Vec3::new(10.0, 0.0, 30.0);
        car.step(&DriveInput { brake: 0.25, ..Default::default() }, &ground, None);
        let supported: Vec<_> = (0..4).map(|i| car.vehicle.wheel(i)).filter(|w| w.on_ground).collect();
        assert!(!supported.is_empty());
        let all_zero_slip = supported.iter().all(|w| w.slip == 0.0);
        assert_eq!(all_zero_slip, destroyed && cop && !player, "role {player}/{cop}/{destroyed}");
    }
}

fn rail() -> CollisionWorld {
    use blackbox_collision::{Article, BARRIER_TWO_SIDED, Barrier, CollisionPack, Instance};
    let mut world = CollisionWorld::new(None);
    world.insert(CollisionPack {
        section: 7,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            half_width: 10.0,
            half_height: 4.0,
            half_length: 10.0,
            translation: [0.0; 3],
            radius: 16.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![],
            barriers: vec![Barrier {
                p0: [0.8, -3.0, 8.0],
                p1: [0.8, 3.0, -8.0],
                surface: 0,
                flags: BARRIER_TWO_SIDED,
                inv_xz_len: 1.0 / 16.0,
            }],
            surfaces: vec![0],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: vec![],
    });
    world
}

#[test]
#[ignore = "needs installed cop physics; CPU-only post-barrier coasting comparison"]
fn installed_coasting_cops_compare_native_and_racer_after_the_same_oblique_rail_hit() {
    use blackbox_vehicle::{FIXED_STEP, InputState};
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = nfsmw_data::car::physics::PhysicsData::load(&dir).unwrap();
    let ground = FlatGround::new(0.0);
    let rail = rail();
    for name in ["copmidsize", "copgto", "copsport", "copsuv"] {
        let mut car = CarSim::new(data.car(name).unwrap(), [0.3; 4]);
        car.configure_ai();
        car.collision_role.cop = true;
        assert!(car.place(&ground, Spawn { position: Vec3::ZERO, heading: 0.0 }));
        for _ in 0..120 {
            car.step(&DriveInput::default(), &ground, None);
        }
        car.set_destroyed_non_player(true);
        car.body_mut().linear_velocity = Vec3::new(8.0, 0.0, 30.0);
        let impact = walls::resolve_into(
            &mut car.vehicle,
            &walls::world_cast_with_mask(&rail, 0),
            &|_, _, _| vec![],
            &car.walls,
            &|_| 1.0,
            false,
            &mut Vec::new(),
        );
        assert!(impact.impulse > 0.0, "{name}: must resolve a real oblique barrier hit");
        let initial = car.body().linear_velocity;
        let local = car.body().rotation().transpose() * initial;
        assert!(local.x.abs() > 0.1, "post-contact state must include lateral sliding: {local:?}");
        let mut native = car.vehicle.clone();
        let mut racer = native.clone();
        racer.set_coasting_tires(None);
        let mut loaded = 0;
        let mut max_native_energy = 0.0_f32;
        let mut max_racer_energy = 0.0_f32;
        for _ in 0..180 {
            for (vehicle, energy) in [(&mut native, &mut max_native_energy), (&mut racer, &mut max_racer_energy)] {
                let speed = Vec3::new(vehicle.linear_velocity().x, 0.0, vehicle.linear_velocity().z).length();
                let brake = match speed >= 2.5 {
                    true => 0.25,
                    false => 0.0,
                };
                vehicle.step(FIXED_STEP, &InputState { brake, ..Default::default() }, &ground);
                assert!(vehicle.linear_velocity().is_finite() && vehicle.angular_velocity().is_finite());
                let kinetic = 0.5 * vehicle.body().mass() * vehicle.linear_velocity().length_squared();
                assert!(kinetic.is_finite());
                *energy = energy.max(kinetic);
            }
            for wheel in (0..4).map(|i| native.wheel(i)).filter(|w| w.on_ground) {
                loaded += 1;
                assert_eq!(wheel.slip, 0.0);
                assert!(!wheel.locked);
            }
        }
        assert!(loaded > 0, "{name}: coasting must pass through supported physical tires");
        assert!(!native.position().abs_diff_eq(racer.position(), 0.00001), "old and native tire policies must differ");
        eprintln!(
            "{name}: shared post-rail {initial:?}; native/racer max translational energy {max_native_energy}/{max_racer_energy}; final velocity {:?}/{:?}",
            native.linear_velocity(),
            racer.linear_velocity()
        );
    }
}
