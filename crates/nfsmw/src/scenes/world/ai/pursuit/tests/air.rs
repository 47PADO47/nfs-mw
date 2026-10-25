//! The helicopter in the wave: it takes its turn among the cop types once it is allowed, appears on the road behind the
//! player and counts as a cop spawned. Spec: `docs/specs/ai-helicopter.md` section 2.

use super::free_roam::see;
use super::*;

fn chase_with_heli(level: u32) -> TrafficWorld {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let model = cop_model(&mut renderer, "copx", player());
    world.set_helicopter_rig(model.model.rig.clone());
    world.add_cop_model(model);
    let mut row = wave();
    row.cops.push(CopRecord { name: "copheli".into(), count: 1, chance: 60 });
    world.police.set_forced_pursuit(true);
    world.start_pursuit(level, vec![row], player());
    assert_eq!(world.police.level(), Some(level));
    world
}

fn run_manager(world: &mut TrafficWorld, steps: usize) {
    let collision = floor();
    run_manager_on(world, steps, &collision);
}

fn run_manager_on(world: &mut TrafficWorld, steps: usize, collision: &CollisionWorld) {
    for _ in 0..steps {
        world.update_pursuit(0.5, support_focus(0.0), collision, &SurfaceTable::default());
        see(world, true);
        see(world, true);
    }
}

#[test]
fn the_helicopter_waits_for_its_timer_and_the_support_delay() {
    let mut world = chase_with_heli(5);
    run_manager(&mut world, 10);
    assert!(!world.pursuit_feed().unwrap().heli_present, "5 s of chase: the 10 s timer is still running");
    assert!(world.helicopter_status().is_none());
}

#[test]
fn the_helicopter_takes_its_turn_and_appears_on_the_road_behind_the_player() {
    let mut world = chase_with_heli(5);
    run_manager(&mut world, 200);
    let feed = world.pursuit_feed().unwrap();
    assert!(feed.heli_present, "after the 35 s support delay of heat 5 the helicopter is picked");
    let status = world.helicopter_status().expect("flying");
    assert!(status.fuel_secs <= 90.0 && status.fuel_secs > 20.0, "heat 5's tank: {}", status.fuel_secs);
    assert_eq!(status.health, 1.0);
    assert!(feed.heli_fuel > 0.2 && feed.heli_fuel <= 1.0);
    assert!(world.police.cost_to_state() >= 2000, "a helicopter costs the state 2000");
    assert!(!feed.heli_search_call, "it was an ordinary pick, not the search-mode roll");
}

#[test]
fn only_one_helicopter_flies_at_a_time() {
    let mut world = chase_with_heli(5);
    run_manager(&mut world, 120);
    let cost = world.police.cost_to_state();
    run_manager(&mut world, 20);
    assert_eq!(world.police.cost_to_state(), cost, "no second helicopter was sent while the first flies");
}

#[test]
#[ignore = "needs installed pursuit/support rows and vehicle physics via NFSMW_GAME_DIR"]
fn installed_support_rows_automatically_deploy_ram_and_heli_when_unblocked() {
    use game_install::GameDir;
    use nfsmw_data::car::physics::PhysicsData;

    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let collision = super::installed_spawn_drive::corridor();
    for z in [-400.0, -150.0, 270.0, 340.0, 400.0] {
        assert!(
            collision.ray_cast([0.0, 10.0, z], [0.0, -10.0, z], &Default::default()).is_some(),
            "fixture must support every deployment site, z={z}"
        );
    }
    let mut observed = Vec::new();
    for level in [3, 4, 5] {
        let mut any_ram = false;
        let mut any_heli = false;
        for seed in [super::super::super::streams::WORLD_SEED, 0, 1] {
            let mut world = TrafficWorld::new(straight_road(2000.0));
            world.reseed(seed);
            world.load_data(data.database(), None);
            let rows = nfsmw_data::pursuit::heat_rows(data.database());
            let mut names = nfsmw_data::pursuit::wave_names(&rows);
            names.extend(["copsuvl".into(), "copsuv".into(), "copcross".into()]);
            for name in names {
                if world.has_cop_model(&name) {
                    continue;
                }
                let mut model = cop_model(&mut free_roam::renderer(), &name, player());
                model.model.physics = data.car(&name).unwrap();
                world.add_cop_model(model);
            }
            let rig = world.cop_models[0].model.rig.clone();
            world.set_helicopter_rig(rig);
            world.police.set_forced_pursuit(true);
            world.start_pursuit(level, rows, player());
            let mut ram = false;
            let mut heli = false;
            let mut ram_order_samples = 0;
            for _ in 0..480 {
                run_manager_on(&mut world, 1, &collision);
                ram |= world.cars.iter().any(AiCar::head_on_support);
                heli |= world.police.helicopter_present();
                ram_order_samples += usize::from(
                    world
                        .pursuit
                        .as_ref()
                        .unwrap()
                        .support
                        .order()
                        .is_some_and(|order| order.strategy() == Strategy::Ram),
                );
            }
            eprintln!(
                "AUTOMATIC_SUPPORT heat={level} seed={seed} ram={ram} heli={heli} ram_order_samples={ram_order_samples}; {:?}; {}",
                world.police.support_row(level),
                world.pursuit_status()
            );
            any_ram |= ram;
            any_heli |= heli;
        }
        observed.push((level, any_ram, any_heli));
    }
    for (level, ram, heli) in observed {
        assert!(
            ram,
            "unblocked installed heat {level} must exercise automatic ram selection and admission across the fixed seed matrix"
        );
        if level >= 4 {
            assert!(heli, "unblocked installed heat {level} must exercise automatic helicopter deployment");
        }
    }
}

#[test]
fn whenever_a_helicopter_flies_its_blip_reaches_the_world_maps_moving_cars_at_every_heat() {
    use crate::frontend::world_map::overlay::live;
    use crate::hud::HELICOPTER;
    let city = blackbox_minimap::Calibration { origin: [-1224.8894, -1591.1449], width: 6659.332 };
    let mut flew_at = Vec::new();
    for level in 1..=10 {
        let mut world = chase_with_heli(level);
        run_manager(&mut world, 200);
        let mut blips = Vec::new();
        world.map_blips(&mut blips);
        let on_hud = blips.iter().filter(|b| b.style == HELICOPTER).count();
        let on_map = live(&blips, &city).iter().filter(|b| b.style == HELICOPTER).count();
        assert_eq!(
            on_hud,
            usize::from(world.helicopter_status().is_some()),
            "heat {level}: the HUD has a blip while it flies"
        );
        assert_eq!(on_map, on_hud, "heat {level}: the map's moving cars have it too");
        if on_map == 1 {
            flew_at.push(level);
        }
    }
    eprintln!("the helicopter flew at heats {flew_at:?}");
    assert!(flew_at.len() >= 3, "the helicopter flew at heats {flew_at:?}");
}
