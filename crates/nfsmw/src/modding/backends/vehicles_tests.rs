//! `bb.vehicles` on the game side: the state is the car the bridge read at the sync, a move and a repair are queued for the
//! scene in order, and a traffic spawn is unsupported.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_NOT_READY, BB_ERR_UNSUPPORTED};
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
fn spawning_traffic_is_unsupported_in_1_0() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::driving([0.0; 3], 0.0));
    assert_eq!(port.spawn_traffic("example.a", [0.0; 3], 0.0), Err(BB_ERR_UNSUPPORTED));
}
