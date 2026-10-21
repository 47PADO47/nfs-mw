//! Real police heat growth and world wave selection, independent of HUD state.

use super::*;
use crate::scenes::world::ai::police::{PlayerView, Sensed};

#[test]
fn natural_heat_growth_changes_the_world_wave_after_one_configured_level() {
    let mut second = wave();
    second.cops[0].name = "copghost".into();
    let mut world = TrafficWorld::new(straight_road(300.0));
    world.start_pursuit(1, vec![wave(), second], player());
    assert_eq!(world.police.settings().time_per_level(1), 600.0);
    let view = PlayerView { position: Vec3::ZERO, forward: Vec3::X, speed: 25.0, nitrous: 0.0, throttle: 0.5 };
    let sensed = Sensed {
        seen: true,
        unit_random: 0.5,
        on_road: true,
        nearest_cop: 10.0,
        hit_traffic: false,
        struck_cop: false,
        struck_property: false,
        hidden: false,
        hidden_from_heli: false,
        in_tunnel: false,
        formation_random: [0.5; 2],
        formation_gap: None,
        support_random: 0.5,
    };
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let dt = crate::scenes::world::drive::STEP;
    for _ in 0..(599 * 60) {
        assert!(!world.police.update(dt, true, &view, sensed));
    }
    world.update_pursuit(dt, focus(), &collision, &surfaces);
    assert_eq!(world.police.level(), Some(1));
    assert_eq!(world.pursuit.as_ref().unwrap().wave().next().unwrap().0, "copx");
    // Float additions can cross a few ticks after exactly 600 seconds; no HUD
    // percentage or display rounding is allowed to choose the gameplay level.
    for _ in 0..(2 * 60) {
        assert!(!world.police.update(dt, true, &view, sensed));
    }
    world.update_pursuit(dt, focus(), &collision, &surfaces);
    assert_eq!(world.police.level(), Some(2));
    assert!((world.police.heat_value() - (1.0 + 601.0 / 600.0)).abs() < 0.001);
    assert_eq!(world.pursuit.as_ref().unwrap().heat, 2);
    assert_eq!(world.pursuit.as_ref().unwrap().wave().next().unwrap().0, "copghost");
}
