//! A heat rise changes the wave's car types (`docs/specs/ai-pursuit-cops.md` section 4, "Wave re-pick on a heat rise"):
//! the cops of the old type are sent away as surplus, and a cop sent away leaves the chase at once (section 5), so the new
//! type comes in while the old cops are still driving off.

use super::free_roam::renderer;
use super::*;

/// Two cops of type `copx` in a chase on a 2 km road; heat 1 wants two `copx`, heat 2 wants two `copy`.
fn setup() -> (TrafficWorld, Focus) {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    world.add_cop_model(cop_model(&mut renderer(), "copy", player()));
    world.configure(20, 8);
    let body = Body {
        position: Vec3::new(0.0, 0.0, -200.0),
        velocity: Vec3::ZERO,
        forward: Vec2::Y,
        half_width: 1.0,
        half_length: 2.0,
    };
    let focus = Focus { position: [-200.0, 0.0], heading: 0.0, speed: 0.0, body: Some(body) };
    for z in [-180.0, -170.0] {
        super::free_roam::patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, z));
    }
    world.player_position = Some(body.position);
    let row = |name: &str| {
        let mut row = wave();
        row.cops = vec![CopRecord { name: name.into(), count: 2, chance: 100 }];
        row.full_engagement_cops = 20;
        row
    };
    world.start_pursuit(1, vec![row("copx"), row("copy")], player());
    world.pursuit.as_mut().unwrap().opening = None;
    (world, focus)
}

fn count(world: &TrafficWorld, name: &str, fleeing: bool) -> usize {
    world.cars.iter().filter(|car| car.is_cop() && car.name == name && car.is_fleeing() == fleeing).count()
}

#[test]
fn a_heat_rise_brings_the_new_cop_type_while_the_old_cops_drive_away() {
    let (mut world, focus) = setup();
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    assert_eq!(count(&world, "copx", false), 2, "both patrols joined the chase");
    for _ in 0..30 {
        world.update_pursuit(FRAME, focus, &collision, &surfaces);
    }
    assert_eq!(count(&world, "copy", false), 0, "heat 1 wants no copy");
    world.police.set_heat(2.0);
    assert_eq!(world.police.level(), Some(2));
    for _ in 0..90 {
        world.update_pursuit(FRAME, focus, &collision, &surfaces);
    }
    assert_eq!(count(&world, "copx", true), 2, "the old type is sent away");
    assert!(count(&world, "copy", false) >= 1, "the new type comes in while the old cops are still on the road");
}
