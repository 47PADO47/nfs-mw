//! The suspension (1.1) and controls (1.2) hooks of `bb.handling` on the game side: each goes on the player's car on its own, is
//! taken off on unregister, quarantine and an online session, is refused online at registration, and a mod's faults across the
//! three kinds are counted for the runtime.

use std::rc::Rc;

use blackbox_modding::abi::{
    BB_CONTROLS_FLAG_STEER, BB_CONTROLS_HANDLED, BB_ERR_DENIED, BB_ERR_EXISTS, BB_ERR_PANIC, BB_ERR_STALE,
    BB_SUSPENSION_HANDLED, BB_TYRE_HANDLED, BbControlsQuery, BbControlsResult, BbStatus, BbSuspensionQuery,
    BbSuspensionResult, BbSuspensionWheelResult, BbTyreQuery, BbTyreResult,
};
use blackbox_modding::host::{ControlsHook, HandlingBackend, SuspensionHook, TyreHook};
use blackbox_vehicle::input::ControlsQuery;
use blackbox_vehicle::suspension::SuspensionQuery;
use blackbox_vehicle::{Vehicle, VehicleSpec};

use super::bridge::Bridge;
use super::handling::HandlingPort;
use super::support::Recorder;

const OWNER: &str = "example.a";

fn suspension_answers(query: &BbSuspensionQuery, result: &mut BbSuspensionResult) -> BbStatus {
    let wheel = BbSuspensionWheelResult {
        body_force_n: 1000.0 + query.axle_index as f32,
        tyre_load_n: 1200.0,
        camber_rad: -0.02,
        reserved: [0; 2],
    };
    result.wheels = [wheel, wheel];
    BB_SUSPENSION_HANDLED
}

fn controls_answers(query: &BbControlsQuery, result: &mut BbControlsResult) -> BbStatus {
    result.throttle = query.throttle * 0.5;
    result.brake = [0.25; 4];
    result.flags = BB_CONTROLS_FLAG_STEER;
    result.steer_target_rad = 0.1;
    BB_CONTROLS_HANDLED
}

fn tyre_answers(_: &BbTyreQuery, result: &mut BbTyreResult) -> BbStatus {
    result.traction = 1.0;
    BB_TYRE_HANDLED
}

fn suspension_faults(_: &BbSuspensionQuery, _: &mut BbSuspensionResult) -> BbStatus {
    BB_ERR_PANIC
}

fn controls_faults(_: &BbControlsQuery, _: &mut BbControlsResult) -> BbStatus {
    BB_ERR_PANIC
}

fn fixture() -> (Rc<Bridge>, HandlingPort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), HandlingPort(bridge))
}

fn scene() -> Recorder {
    let mut scene = Recorder::driving([0.0; 3], 0.0);
    scene.summary = Some(Vehicle::new(VehicleSpec::example()).summary());
    scene
}

fn suspension_query() -> SuspensionQuery {
    SuspensionQuery { axle: 1, dt: 1.0 / 60.0, ..SuspensionQuery::default() }
}

fn controls_query() -> ControlsQuery {
    ControlsQuery { dt: 1.0 / 60.0, throttle: 1.0, ..ControlsQuery::default() }
}

#[test]
fn each_kind_goes_on_the_car_on_its_own_and_answers_for_the_player() {
    let (bridge, port) = fixture();
    port.register_suspension(OWNER, SuspensionHook::from_fn(OWNER, suspension_answers)).expect("registered");
    let mut driving = scene();
    bridge.sync(&mut driving);
    assert_eq!((driving.suspension_installs, driving.controls_installs, driving.tyre_installs), (1, 0, 0));
    let answer = driving.suspension_model.as_mut().expect("on the car").axle(&suspension_query()).expect("handled");
    assert_eq!((answer.wheels[0].body_force_n, answer.wheels[1].tyre_load_n), (1001.0, 1200.0));

    port.register_controls(OWNER, ControlsHook::from_fn(OWNER, controls_answers)).expect("registered");
    bridge.sync(&mut driving);
    assert_eq!(
        (driving.suspension_installs, driving.controls_installs),
        (1, 1),
        "a car with the model is not given another"
    );
    let answer = driving.controls_model.as_mut().expect("on the car").filter(&controls_query()).expect("handled");
    assert_eq!((answer.throttle, answer.brake, answer.steer_target), (0.5, [0.25; 4], Some(0.1)));
    assert!(driving.tyre_model.is_none(), "the tyre hook was not registered");
}

#[test]
fn a_mod_may_register_all_three_and_remove_them_one_by_one() {
    let (bridge, port) = fixture();
    let tyre = port.register_tyre(OWNER, TyreHook::from_fn(OWNER, tyre_answers)).expect("tyre");
    let suspension =
        port.register_suspension(OWNER, SuspensionHook::from_fn(OWNER, suspension_answers)).expect("suspension");
    let controls = port.register_controls(OWNER, ControlsHook::from_fn(OWNER, controls_answers)).expect("controls");
    assert_eq!(bridge.hook_owners().len(), 3);
    let mut driving = scene();
    bridge.sync(&mut driving);
    assert!(driving.tyre_model.is_some() && driving.suspension_model.is_some() && driving.controls_model.is_some());

    assert_eq!(port.unregister_suspension(OWNER, suspension), Ok(()));
    bridge.sync(&mut driving);
    assert!(driving.suspension_model.is_none(), "removing the suspension hook takes only that model off the car");
    assert!(driving.tyre_model.is_some() && driving.controls_model.is_some());
    assert_eq!(port.unregister_suspension(OWNER, suspension), Err(BB_ERR_STALE));
    assert_eq!(port.unregister_controls("example.b", controls), Err(BB_ERR_STALE), "only the owner removes a hook");
    assert_eq!(port.unregister_controls(OWNER, controls), Ok(()));
    assert_eq!(port.unregister_tyre(OWNER, tyre), Ok(()));
    bridge.sync(&mut driving);
    assert!(driving.tyre_model.is_none() && driving.controls_model.is_none());
    assert!(bridge.hook_owners().is_empty());
}

#[test]
fn only_one_hook_of_a_kind_is_kept() {
    let (_bridge, port) = fixture();
    port.register_suspension(OWNER, SuspensionHook::from_fn(OWNER, suspension_answers)).expect("registered");
    assert_eq!(
        port.register_suspension("example.b", SuspensionHook::from_fn("example.b", suspension_answers)),
        Err(BB_ERR_EXISTS)
    );
    assert!(port.register_controls("example.b", ControlsHook::from_fn("example.b", controls_answers)).is_ok());
}

#[test]
fn a_networked_session_refuses_the_new_hooks_and_takes_registered_ones_off() {
    let (bridge, port) = fixture();
    let mut online = scene();
    online.networked = true;
    bridge.sync(&mut online);
    assert_eq!(port.register_suspension(OWNER, SuspensionHook::from_fn(OWNER, suspension_answers)), Err(BB_ERR_DENIED));
    assert_eq!(port.register_controls(OWNER, ControlsHook::from_fn(OWNER, controls_answers)), Err(BB_ERR_DENIED));
    assert!(online.suspension_model.is_none() && online.controls_model.is_none());

    online.networked = false;
    bridge.sync(&mut online);
    port.register_suspension(OWNER, SuspensionHook::from_fn(OWNER, suspension_answers)).expect("offline again");
    port.register_controls(OWNER, ControlsHook::from_fn(OWNER, controls_answers)).expect("offline again");
    bridge.sync(&mut online);
    assert!(online.suspension_model.is_some() && online.controls_model.is_some());
    online.networked = true;
    bridge.sync(&mut online);
    assert!(
        online.suspension_model.is_none() && online.controls_model.is_none(),
        "the session began: built-in code runs"
    );
    bridge.sync(&mut online);
    assert!(online.suspension_model.is_none() && online.controls_model.is_none());
    online.networked = false;
    bridge.sync(&mut online);
    bridge.sync(&mut online);
    assert!(
        online.suspension_model.is_some() && online.controls_model.is_some(),
        "the session ended: the hooks are back"
    );
}

#[test]
fn the_faults_of_every_kind_are_counted_for_the_runtime_and_the_models_leave_the_car() {
    let (bridge, port) = fixture();
    port.register_suspension(OWNER, SuspensionHook::from_fn(OWNER, suspension_faults)).expect("registered");
    port.register_controls(OWNER, ControlsHook::from_fn(OWNER, controls_faults)).expect("registered");
    let mut driving = scene();
    bridge.sync(&mut driving);
    for _ in 0..5 {
        assert!(driving.suspension_model.as_mut().expect("on the car").axle(&suspension_query()).is_none());
        assert!(driving.controls_model.as_mut().expect("on the car").filter(&controls_query()).is_none());
    }
    let mut counted = bridge.take_hook_faults();
    counted.sort();
    assert_eq!(counted, vec![(OWNER.to_owned(), 5), (OWNER.to_owned(), 5)], "one entry per hook");
    assert!(bridge.take_hook_faults().is_empty(), "each fault is handed over once");
    bridge.sync(&mut driving);
    assert!(driving.suspension_model.is_none() && driving.controls_model.is_none(), "quarantined hooks leave the car");
    assert!(bridge.hook_owners().is_empty());
    assert!(
        port.register_suspension(OWNER, SuspensionHook::from_fn(OWNER, suspension_answers)).is_ok(),
        "the slot is free"
    );
}

#[test]
fn a_quarantined_mod_loses_all_its_hooks() {
    let (bridge, port) = fixture();
    port.register_tyre(OWNER, TyreHook::from_fn(OWNER, tyre_answers)).expect("registered");
    port.register_suspension(OWNER, SuspensionHook::from_fn(OWNER, suspension_answers)).expect("registered");
    port.register_controls(OWNER, ControlsHook::from_fn(OWNER, controls_answers)).expect("registered");
    let mut driving = scene();
    bridge.sync(&mut driving);
    bridge.drop_hook_of("example.b");
    assert_eq!(bridge.hook_owners().len(), 3, "another mod's quarantine leaves the hooks");
    bridge.drop_hook_of(OWNER);
    bridge.sync(&mut driving);
    assert!(driving.tyre_model.is_none() && driving.suspension_model.is_none() && driving.controls_model.is_none());
}
