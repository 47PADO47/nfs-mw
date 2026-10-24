//! Actual support acquisition, pending expiry, contact loss, collapse and release through the world adapter.

use super::*;
use blackbox_pursuit::formation::Formation;

fn world() -> TrafficWorld {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.add_cop_model(cop_model(&mut free_roam::renderer(), "copx", player()));
    world.start_pursuit(1, vec![wave()], player());
    world
}

fn active(strategy: Strategy) -> TrafficWorld {
    let mut world = world();
    for name in ["copsuvl", "copcross"] {
        world.add_cop_model(cop_model(&mut free_roam::renderer(), name, player()));
    }
    assert!(world.ask_support(support(strategy)));
    let target = match strategy.is_heavy() {
        true => -150.0,
        false => 0.0,
    };
    world.update_support(FRAME, support_focus(target), &floor(), &SurfaceTable::default());
    assert!(support_order(&world).unwrap().is_active());
    assert_eq!(support_count(&world), 1);
    world
}

#[test]
fn a_pending_model_request_expires_while_searching_and_can_be_requested_again() {
    let mut world = world();
    let request = Request { strategy: Strategy::CrossFollow, duration: 1.0 };
    assert!(world.ask_support(request));
    world.update_support(FRAME, support_focus(0.0), &floor(), &SurfaceTable::default());
    assert!(support_order(&world).unwrap().is_pending());
    assert!(support_order(&world).unwrap().timer() < 1.0);
    world.age_search_support(2.0);
    assert!(support_order(&world).is_none());
    assert_eq!(support_count(&world), 0);
    assert!(world.ask_support(request), "timeout is not a permanent model failure");
}

#[test]
fn active_support_gets_the_full_duration_after_a_delayed_model_arrives() {
    let mut world = world();
    let request = Request { strategy: Strategy::CrossFollow, duration: 5.0 };
    assert!(world.ask_support(request));
    world.update_support(2.0, support_focus(0.0), &floor(), &SurfaceTable::default());
    assert_eq!(support_order(&world).unwrap().timer(), 3.0);
    world.add_cop_model(cop_model(&mut free_roam::renderer(), "copcross", player()));
    world.update_support(FRAME, support_focus(0.0), &floor(), &SurfaceTable::default());
    let order = support_order(&world).unwrap();
    assert!(order.is_active());
    assert_eq!(order.timer(), 5.0, "activation restarts the option duration");
}

#[test]
fn tracked_contact_loss_sends_active_heavy_support_away_with_its_lights_off() {
    let mut world = active(Strategy::EBrake);
    for _ in 0..52 {
        free_roam::see(&mut world, false);
    }
    assert!(!world.police.is_perp_in_sight());
    world.update_pursuit(FRAME, support_focus(0.0), &floor(), &SurfaceTable::default());
    assert!(support_order(&world).is_none());
    let suv = world.cars.iter().find(|car| car.name == "copsuvl").unwrap();
    assert!(suv.is_fleeing());
    assert_eq!(suv.support_serial(), None);
    assert!(!suv.emergency_lights());
}

#[test]
fn a_real_collapse_cancels_active_heavy_support() {
    let mut world = active(Strategy::EBrake);
    assert!(world.spawn_cop_at(
        "copx",
        CopSpot { at: Vec3::new(2.0, 0.0, 14.0), target: Vec3::ZERO, speed: 0.0, player: player(), band: None },
        &floor(),
        &SurfaceTable::default()
    ));
    world.police.test_formation(Formation::BoxIn, false);
    let target = support_focus(0.0).body.unwrap();
    world.cop_aims(&target, &floor(), &mut Vec::new());
    assert!(world.collapse_active());
    world.update_pursuit(FRAME, support_focus(0.0), &floor(), &SurfaceTable::default());
    assert!(support_order(&world).is_none());
    assert!(world.cars.iter().find(|car| car.name == "copsuvl").unwrap().is_fleeing());
}

#[test]
fn leader_support_keeps_its_order_on_contact_loss_then_expires_normally() {
    let mut world = active(Strategy::CrossFollow);
    world.cancel_heavy_support();
    assert!(support_order(&world).unwrap().is_active(), "heavy-support cancellation does not cancel a leader");
    world.age_search_support(31.0);
    assert!(support_order(&world).is_none());
    assert_eq!(support_count(&world), 0);
    assert!(!world.cars.iter().find(|car| car.name == "copcross").unwrap().is_fleeing());
}

#[test]
fn losing_all_support_cars_releases_the_order_before_its_duration_runs_out() {
    let mut world = active(Strategy::CrossFollow);
    world.cars.clear();
    world.update_support(FRAME, support_focus(0.0), &floor(), &SurfaceTable::default());
    assert!(support_order(&world).is_none());
    assert!(world.ask_support(support(Strategy::EBrake)));
}
