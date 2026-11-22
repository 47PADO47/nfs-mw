//! `bb.handling` on the game side: the tyre hook of a mod is put on the player's car at the frame boundary, taken off again on
//! unregister, quarantine and an online session, refused online at registration, and the car's summary reaches `bb.vehicles`
//! `spec` with a revision that follows the car.

use std::rc::Rc;

use blackbox_modding::abi::{
    BB_ERR_DENIED, BB_ERR_EXISTS, BB_ERR_NOT_READY, BB_ERR_PANIC, BB_ERR_STALE, BB_TYRE_HANDLED, BbStatus, BbTyreQuery,
    BbTyreResult,
};
use blackbox_modding::host::{HandlingBackend, TyreHook, VehiclesBackend};
use blackbox_vehicle::tires::TyreQuery;
use blackbox_vehicle::{Vehicle, VehicleSpec};

use super::bridge::Bridge;
use super::handling::HandlingPort;
use super::support::Recorder;
use super::vehicles::VehiclesPort;

const OWNER: &str = "example.a";

fn answers(query: &BbTyreQuery, result: &mut BbTyreResult) -> BbStatus {
    result.wheel_spin = query.role as f32 + 1.0;
    result.traction = 1.0;
    BB_TYRE_HANDLED
}

fn faults(_: &BbTyreQuery, _: &mut BbTyreResult) -> BbStatus {
    BB_ERR_PANIC
}

fn hook_of(call: fn(&BbTyreQuery, &mut BbTyreResult) -> BbStatus, owner: &str) -> TyreHook {
    TyreHook::from_fn(owner, call)
}

fn fixture() -> (Rc<Bridge>, HandlingPort, VehiclesPort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), HandlingPort(bridge.clone()), VehiclesPort(bridge))
}

fn scene() -> Recorder {
    let mut scene = Recorder::driving([0.0; 3], 0.0);
    scene.summary = Some(Vehicle::new(VehicleSpec::example()).summary());
    scene
}

fn query() -> TyreQuery {
    TyreQuery {
        wheel_index: 0,
        role: 0,
        loaded: true,
        dt: 1.0 / 60.0,
        vertical_load_n: 3000.0,
        patch_lateral_velocity: 0.0,
        patch_forward_velocity: 10.0,
        body_speed: 10.0,
        wheel_spin: 30.0,
        drive_torque_nm: 0.0,
        brake_torque_nm: 0.0,
        wheel_radius_m: 0.33,
        wheel_inertia_kgm2: 10.0,
        corner_mass_kg: 375.0,
        surface_lateral: 1.0,
        surface_drive: 1.0,
        surface_rolling: 1.0,
        surface_tag: 0,
        tyre_condition: 1.0,
        ambient_temperature_c: 20.0,
        road_temperature_c: 25.0,
        camber_rad: 0.0,
    }
}

#[test]
fn the_spec_is_unavailable_until_a_car_is_driven_and_follows_the_scene() {
    let (bridge, _port, vehicles) = fixture();
    bridge.sync(&mut Recorder::without_clock());
    assert_eq!(vehicles.spec(OWNER), Err(BB_ERR_NOT_READY));
    let mut driving = scene();
    bridge.sync(&mut driving);
    let sheet = vehicles.spec(OWNER).expect("a car is driven");
    assert_eq!(sheet.revision, 1);
    assert!(sheet.mass_kg > 500.0 && sheet.wheelbase_m > 2.0 && sheet.gear_count >= 4);
    assert_eq!(sheet.downforce_area_m2, 0.0);
    bridge.sync(&mut driving);
    assert_eq!(vehicles.spec(OWNER).expect("still driven").revision, 1, "the same car keeps its revision");
    driving.summary.as_mut().expect("summary").mass_kg += 100.0;
    bridge.sync(&mut driving);
    assert_eq!(vehicles.spec(OWNER).expect("driven").revision, 2, "a changed car is a new revision");
}

#[test]
fn a_registered_hook_goes_on_the_car_and_answers_for_the_player() {
    let (bridge, port, _vehicles) = fixture();
    let handle = port.register_tyre(OWNER, hook_of(answers, OWNER)).expect("registered");
    let mut driving = scene();
    bridge.sync(&mut driving);
    assert_eq!(driving.tyre_installs, 1);
    let model = driving.tyre_model.as_mut().expect("the model is on the car");
    let answer = model.wheel(&query()).expect("handled");
    assert_eq!(answer.wheel_spin, 1.0, "the player's role tag is 0");
    bridge.sync(&mut driving);
    assert_eq!(driving.tyre_installs, 1, "a car that has the model is not given another");
    assert_eq!(port.unregister_tyre(OWNER, handle), Ok(()));
    bridge.sync(&mut driving);
    assert!(driving.tyre_model.is_none(), "unregistering takes the model off at the next frame boundary");
    assert_eq!(port.unregister_tyre(OWNER, handle), Err(BB_ERR_STALE));
}

#[test]
fn a_new_car_gets_the_model_too() {
    let (bridge, port, _vehicles) = fixture();
    port.register_tyre(OWNER, hook_of(answers, OWNER)).expect("registered");
    let mut driving = scene();
    bridge.sync(&mut driving);
    driving.tyre_model = None;
    bridge.sync(&mut driving);
    assert_eq!(driving.tyre_installs, 2, "the scene put the model on the car again");
}

#[test]
fn only_one_hook_is_kept_and_only_its_owner_removes_it() {
    let (_bridge, port, _vehicles) = fixture();
    let handle = port.register_tyre(OWNER, hook_of(answers, OWNER)).expect("registered");
    assert_eq!(port.register_tyre("example.b", hook_of(answers, "example.b")), Err(BB_ERR_EXISTS));
    assert_eq!(port.unregister_tyre("example.b", handle), Err(BB_ERR_STALE));
    assert_eq!(port.unregister_tyre(OWNER, handle), Ok(()));
}

#[test]
fn a_networked_session_refuses_the_hook() {
    let (bridge, port, _vehicles) = fixture();
    let mut online = scene();
    online.networked = true;
    bridge.sync(&mut online);
    assert_eq!(port.register_tyre(OWNER, hook_of(answers, OWNER)), Err(BB_ERR_DENIED));
    assert!(online.tyre_model.is_none());
    online.networked = false;
    bridge.sync(&mut online);
    assert!(port.register_tyre(OWNER, hook_of(answers, OWNER)).is_ok(), "the refusal left nothing behind");
}

#[test]
fn a_hook_registered_before_a_session_is_off_while_it_lasts() {
    let (bridge, port, _vehicles) = fixture();
    port.register_tyre(OWNER, hook_of(answers, OWNER)).expect("registered");
    let mut driving = scene();
    bridge.sync(&mut driving);
    assert!(driving.tyre_model.is_some());
    driving.networked = true;
    bridge.sync(&mut driving);
    assert!(driving.tyre_model.is_none(), "the session began: the built-in tyres run");
    bridge.sync(&mut driving);
    assert!(driving.tyre_model.is_none());
    driving.networked = false;
    bridge.sync(&mut driving);
    bridge.sync(&mut driving);
    assert!(driving.tyre_model.is_some(), "the session ended: the hook is back");
}

#[test]
fn a_quarantined_hook_is_counted_for_the_runtime_and_taken_off_the_car() {
    let (bridge, port, _vehicles) = fixture();
    port.register_tyre(OWNER, hook_of(faults, OWNER)).expect("registered");
    let mut driving = scene();
    bridge.sync(&mut driving);
    for _ in 0..5 {
        assert!(driving.tyre_model.as_mut().expect("on the car").wheel(&query()).is_none());
    }
    assert_eq!(bridge.take_hook_faults(), vec![(OWNER.to_owned(), 5)]);
    assert!(bridge.take_hook_faults().is_empty(), "each fault is handed over once");
    bridge.sync(&mut driving);
    assert!(driving.tyre_model.is_none(), "a quarantined hook leaves the car");
    assert!(bridge.hook_owners().is_empty());
    assert!(port.register_tyre(OWNER, hook_of(answers, OWNER)).is_ok(), "the slot is free again");
}

#[test]
fn a_quarantined_mod_loses_its_hook() {
    let (bridge, port, _vehicles) = fixture();
    port.register_tyre(OWNER, hook_of(answers, OWNER)).expect("registered");
    let mut driving = scene();
    bridge.sync(&mut driving);
    bridge.drop_hook_of("example.b");
    assert!(!bridge.hook_owners().is_empty(), "another mod's quarantine leaves the hook");
    bridge.drop_hook_of(OWNER);
    bridge.sync(&mut driving);
    assert!(driving.tyre_model.is_none());
}
