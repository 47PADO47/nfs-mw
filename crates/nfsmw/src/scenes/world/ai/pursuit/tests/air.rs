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
    world.start_pursuit(level, vec![row], player());
    world
}

fn run_manager(world: &mut TrafficWorld, steps: usize) {
    let collision = floor();
    for _ in 0..steps {
        world.update_pursuit(0.5, support_focus(0.0), &collision, &SurfaceTable::default());
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
