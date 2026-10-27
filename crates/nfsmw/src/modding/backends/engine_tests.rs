//! `bb.engine` on the game side: the state and the car id come from the bridge's copy of the scene, a stop and a start are
//! queued for the scene, and the engine a mod stopped runs again when that mod unloads.

use std::rc::Rc;

use blackbox_modding::abi::BB_ERR_NOT_READY;
use blackbox_modding::host::{EngineBackend, EnginePhase, EngineState};

use super::bridge::Bridge;
use super::engine::EnginePort;
use super::world::WorldRequest;
use crate::engine_switch::{EngineCommand, EngineReadout, StartSound};

fn port() -> (Rc<Bridge>, EnginePort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), EnginePort(bridge))
}

fn driving(bridge: &Bridge) {
    let state = EngineState { phase: EnginePhase::Running, free_roam: true, throttle: 0.4, brake: 0.0, speed: 2.5 };
    *bridge.engine.borrow_mut() = Some(EngineReadout { state, car: "POR911CARR12".to_owned() });
}

fn requests(bridge: &Bridge) -> Vec<WorldRequest> {
    std::mem::take(&mut *bridge.world_requests.borrow_mut())
}

#[test]
fn nothing_is_available_while_no_car_is_driven() {
    let (bridge, port) = port();
    assert_eq!(port.state("example.a"), Err(BB_ERR_NOT_READY));
    assert_eq!(port.car("example.a"), Err(BB_ERR_NOT_READY));
    assert_eq!(port.stop("example.a"), Err(BB_ERR_NOT_READY));
    assert_eq!(port.start("example.a", "", 1.0, 0.0), Err(BB_ERR_NOT_READY));
    assert!(requests(&bridge).is_empty());
}

#[test]
fn the_state_and_the_car_come_from_the_sync() {
    let (bridge, port) = port();
    driving(&bridge);
    let state = port.state("example.a").unwrap();
    assert_eq!((state.phase, state.free_roam, state.throttle, state.speed), (EnginePhase::Running, true, 0.4, 2.5));
    assert_eq!(port.car("example.a").as_deref(), Ok("POR911CARR12"));
}

#[test]
fn a_stop_and_a_start_are_queued_for_the_scene() {
    let (bridge, port) = port();
    driving(&bridge);
    port.stop("example.a").unwrap();
    port.start("example.a", "SOUND/MODENGINE/CAR/IGNITION.WAV", 0.8, 0.6).unwrap();
    port.start("example.a", "", 1.0, 0.0).unwrap();
    let sound = StartSound { file: "SOUND/MODENGINE/CAR/IGNITION.WAV".to_owned(), gain: 0.8 };
    assert_eq!(
        requests(&bridge),
        [
            WorldRequest::Engine(EngineCommand::Stop),
            WorldRequest::Engine(EngineCommand::Start { sound: Some(sound), delay: 0.6 }),
            WorldRequest::Engine(EngineCommand::Start { sound: None, delay: 0.0 }),
        ]
    );
}

#[test]
fn an_engine_the_mod_stopped_runs_again_when_the_mod_unloads() {
    let (bridge, port) = port();
    driving(&bridge);
    port.release("example.a");
    assert!(requests(&bridge).is_empty(), "a mod that stopped nothing changes nothing");
    port.stop("example.a").unwrap();
    requests(&bridge);
    port.release("example.b");
    assert!(requests(&bridge).is_empty(), "another mod's unloading does not start it");
    port.release("example.a");
    assert_eq!(requests(&bridge), [WorldRequest::Engine(EngineCommand::Start { sound: None, delay: 0.0 })]);
    port.release("example.a");
    assert!(requests(&bridge).is_empty(), "once");
}

#[test]
fn a_mod_that_started_the_engine_again_has_nothing_to_release() {
    let (bridge, port) = port();
    driving(&bridge);
    port.stop("example.a").unwrap();
    port.start("example.a", "", 1.0, 0.0).unwrap();
    requests(&bridge);
    port.release("example.a");
    assert!(requests(&bridge).is_empty());
}
