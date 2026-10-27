//! `bb.traffic` on the game side: the pool is the published traffic cars with their numbers, an ask needs a world and a car of
//! the pool, and a good ask goes into the queue the traffic manager serves.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_NOT_FOUND, BB_ERR_NOT_READY};
use blackbox_modding::host::TrafficBackend;
use nfsmw_data::car::mods::{CarSpec, TrafficClass, TrafficUnit, register};

use super::bridge::Bridge;
use super::traffic::{TrafficPort, info_of};
use crate::viewer::WorldInfo;

fn port() -> (Rc<Bridge>, TrafficPort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), TrafficPort(bridge))
}

fn in_a_world(bridge: &Bridge) {
    *bridge.world.borrow_mut() = Some(WorldInfo { track: "Bayview".to_owned(), section: None });
}

/// A mod car that is only traffic, as the game's car table admits it.
fn truck() -> nfsmw_data::car::mods::ModCar {
    let unit = TrafficUnit {
        class: TrafficClass::Truck,
        weight: 4,
        max_instances: 1,
        speed_factor: 0.85,
        patterns: vec!["cityhighway".to_owned(), "industrial".to_owned()],
    };
    let spec = CarSpec {
        folder: "GMCTOPKIC_TR".to_owned(),
        owner: "example.pack".to_owned(),
        name: None,
        manufacturer: None,
        player: false,
        cop: None,
        traffic: Some(unit),
        geometry_in_mod: true,
        stats: Default::default(),
        engine_sound: None,
        car_effects: None,
    };
    register(&[], &[spec]).0.remove(0)
}

#[test]
fn a_pool_entry_carries_the_numbers_of_its_car() {
    let info = info_of(&truck()).unwrap();
    assert_eq!(
        (info.id.as_str(), info.class.as_str(), info.weight, info.max_instances),
        ("GMCTOPKIC_TR", "truck", 4, 1)
    );
    assert_eq!(info.speed_factor, 0.85);
    assert_eq!(info.patterns, ["cityhighway", "industrial"]);
    assert_eq!(info.line(), "GMCTOPKIC_TR\ttruck\t4\t1\t0.85\tcityhighway,industrial");
}

#[test]
fn a_car_that_is_not_traffic_is_not_in_the_pool() {
    let mut car = truck();
    car.traffic = None;
    assert!(info_of(&car).is_none());
}

#[test]
fn nothing_can_be_asked_for_while_no_world_is_loaded() {
    let (_bridge, port) = port();
    assert_eq!(port.spawn("example.a", "GMCTOPKIC_TR", [0.0, 0.0], 0.0), Err(BB_ERR_NOT_READY));
}

#[test]
fn a_car_that_is_not_in_the_pool_is_not_found() {
    let (bridge, port) = port();
    in_a_world(&bridge);
    assert_eq!(port.spawn("example.a", "NOSUCHCAR_TR", [0.0, 0.0], 0.0), Err(BB_ERR_NOT_FOUND));
    assert_eq!(
        port.spawn("example.a", "BMWM3GTR", [0.0, 0.0], 0.0),
        Err(BB_ERR_NOT_FOUND),
        "a stock car is no mod traffic car"
    );
}
