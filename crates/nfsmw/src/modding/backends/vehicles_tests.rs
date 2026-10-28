//! `bb.vehicles` on the game side: the state is the car the bridge read at the sync, a move, a repair and a traffic spawn
//! are queued for the scene in order, and a spawn needs a loaded world.

use std::rc::Rc;

use blackbox_modding::abi::BB_ERR_NOT_READY;
use blackbox_modding::host::VehiclesBackend;

use super::bridge::Bridge;
use super::support::Recorder;
use super::vehicles::VehiclesPort;
use super::world::apply;

fn port() -> (Rc<Bridge>, VehiclesPort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), VehiclesPort(bridge))
}

#[test]
fn the_car_is_unavailable_until_a_car_is_driven() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::without_clock());
    assert_eq!(port.state("example.a"), Err(BB_ERR_NOT_READY));
    assert_eq!(port.teleport("example.a", [0.0; 3], 0.0), Err(BB_ERR_NOT_READY));
    assert_eq!(port.repair("example.a"), Err(BB_ERR_NOT_READY));
    assert!(bridge.world_requests.borrow().is_empty(), "a refused call queues nothing");
}

#[test]
fn the_state_is_the_car_read_at_the_sync() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::driving([10.0, -4.0, 0.5], 1.0));
    let state = port.state("example.a").expect("a car is driven");
    assert_eq!(state.position, [10.0, -4.0, 0.5]);
    assert_eq!(state.velocity, [20.0, 0.0, 0.0]);
    assert_eq!((state.gear, state.rpm, state.speed), (3, 3500.0, 20.0));
    assert_eq!((state.heading, state.nos, state.damage), (1.0, 0.25, 0.0));
    assert_eq!((state.slip_angle, state.steering, state.impact, state.on_ground), (0.0, 0.0, 0.0, true));
}

#[test]
fn the_dynamics_of_the_car_reach_the_mods_as_the_scene_reports_them() {
    let (bridge, port) = port();
    let mut scene = Recorder::driving([0.0; 3], 0.0);
    if let Some(car) = scene.car.as_mut() {
        car.slip_angle = 0.3;
        car.steering = -0.5;
        car.impact = 0.8;
        car.on_ground = false;
    }
    bridge.sync(&mut scene);
    let state = port.state("example.a").expect("a car is driven");
    assert_eq!((state.slip_angle, state.steering, state.impact, state.on_ground), (0.3, -0.5, 0.8, false));
}

#[test]
fn nitrous_is_queued_for_the_scene_and_needs_a_car() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::without_clock());
    assert_eq!(port.add_nos("example.a", 0.1), Err(BB_ERR_NOT_READY));
    assert!(bridge.world_requests.borrow().is_empty(), "a refused call queues nothing");

    bridge.sync(&mut Recorder::driving([0.0; 3], 0.0));
    port.add_nos("example.a", 0.1).expect("a car is driven");
    port.add_nos("example.a", 0.25).expect("a car is driven");
    let requests = std::mem::take(&mut *bridge.world_requests.borrow_mut());
    let mut scene = Recorder::driving([0.0; 3], 0.0);
    apply(requests, &mut scene);
    assert_eq!(scene.nitrous, vec![0.1, 0.25]);
}

#[test]
fn the_drift_assist_is_queued_for_the_scene_and_needs_a_car() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::without_clock());
    assert_eq!(port.set_drift_assist("example.a", true), Err(BB_ERR_NOT_READY));
    assert!(bridge.world_requests.borrow().is_empty(), "a refused call queues nothing");

    bridge.sync(&mut Recorder::driving([0.0; 3], 0.0));
    port.set_drift_assist("example.a", true).expect("a car is driven");
    port.set_drift_assist("example.a", false).expect("a car is driven");
    let requests = std::mem::take(&mut *bridge.world_requests.borrow_mut());
    let mut scene = Recorder::driving([0.0; 3], 0.0);
    apply(requests, &mut scene);
    assert_eq!(scene.drift_assist, vec![true, false]);
}

#[test]
fn a_move_and_a_repair_reach_the_scene_in_order() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::driving([0.0; 3], 0.0));
    port.teleport("example.a", [30.0, 5.0, 0.0], 0.5).expect("a car is driven");
    port.repair("example.a").expect("a car is driven");
    let requests = std::mem::take(&mut *bridge.world_requests.borrow_mut());
    let mut scene = Recorder::driving([0.0; 3], 0.0);
    apply(requests, &mut scene);
    assert_eq!(scene.moves, vec![([30.0, 5.0], 0.5)]);
    assert_eq!(scene.repairs, 1);
}

#[test]
fn a_traffic_spawn_needs_a_world_and_is_queued_with_a_handle() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::without_clock());
    assert_eq!(port.spawn_traffic("example.a", [0.0; 3], 0.0), Err(BB_ERR_NOT_READY));
    assert!(bridge.world_requests.borrow().is_empty(), "a refused spawn queues nothing");

    bridge.sync(&mut Recorder::driving([0.0; 3], 0.0));
    let first = port.spawn_traffic("example.a", [40.0, -12.0, 3.0], 1.5).expect("a world is loaded");
    let second = port.spawn_traffic("example.a", [50.0, 0.0, 0.0], 0.0).expect("a world is loaded");
    assert!(second > first, "each spawn has its own handle");
    let mut scene = Recorder::driving([0.0; 3], 0.0);
    apply(std::mem::take(&mut *bridge.world_requests.borrow_mut()), &mut scene);
    assert_eq!(scene.spawns, vec![([40.0, -12.0], 1.5), ([50.0, 0.0], 0.0)]);
}
