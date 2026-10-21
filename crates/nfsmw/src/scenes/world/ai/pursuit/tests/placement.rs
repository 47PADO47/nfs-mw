//! Where a cop may stand once it is on the road: the 150 to 400 m window and the no-cop-spawn zones
//! (`docs/specs/ai-pursuit-cops.md` section 3.2).

use super::free_roam::renderer;
use super::*;
use crate::scenes::world::ai::cop_manager::square_zone;

fn world_with_a_cop_model() -> TrafficWorld {
    let mut world = TrafficWorld::new(cop_road());
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    world
}

fn spot(band: Option<(f32, f32)>) -> CopSpot {
    CopSpot { at: Vec3::ZERO, target: Vec3::new(0.0, 0.0, 70.0), speed: START_SPEED, player: player(), band }
}

#[test]
fn a_wave_cop_that_snaps_onto_a_road_inside_the_window_is_not_placed() {
    let mut world = world_with_a_cop_model();
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    // The road is 140 m long and the target stands on it: the snapped point is 70 m away, inside the 150 m window.
    assert!(!world.spawn_cop_at("copx", spot(Some((150.0, 400.0))), &collision, &surfaces));
    assert!(world.cars.is_empty());
    assert!(
        world.spawn_cop_at("copx", spot(None), &collision, &surfaces),
        "the forced placements have their own rules"
    );
}

#[test]
fn a_cop_is_not_placed_in_a_no_cop_spawn_zone() {
    let mut world = world_with_a_cop_model();
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    world.cops.set_no_spawn_zones(vec![square_zone(30.0)]);
    assert!(!world.spawn_cop_at("copx", spot(None), &collision, &surfaces));
    world.cops.set_no_spawn_zones(Vec::new());
    assert!(world.spawn_cop_at("copx", spot(None), &collision, &surfaces));
}
