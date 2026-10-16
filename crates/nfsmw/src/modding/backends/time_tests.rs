use std::rc::Rc;

use blackbox_modding::abi::BB_ERR_NOT_READY;
use blackbox_modding::host::TimeBackend;
use nfsmw_data::day_night::{self, DayNightConfig};

use super::bridge::Bridge;
use super::support::Recorder;
use super::time::{self, TimePort};

fn port() -> (Rc<Bridge>, TimePort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), TimePort(bridge))
}

/// Make the scene's requests land on `scene`, as the frame's apply does.
fn apply(bridge: &Bridge, scene: &mut Recorder) {
    let requests = std::mem::take(&mut *bridge.clock_requests.borrow_mut());
    time::apply(requests, scene);
}

#[test]
fn the_clock_is_unavailable_where_the_scene_has_none() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::without_clock());
    assert_eq!(port.get("example.a").map(|_| ()), Err(BB_ERR_NOT_READY));
    assert_eq!(port.set_hour("example.a", 3.0), Err(BB_ERR_NOT_READY));
    assert!(bridge.clock_requests.borrow().is_empty(), "a refused call queues nothing");
}

#[test]
fn get_reads_the_scene_clock_at_the_sync() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::at(13.5, 150.0));
    let clock = port.get("example.a").unwrap();
    assert_eq!((clock.hour, clock.seconds_per_hour, clock.frozen), (13.5, 150.0, 0));
    let lights = day_night::lights_on(13.5, &DayNightConfig::default());
    assert_eq!(clock.lights_on, u8::from(lights));
}

#[test]
fn set_hour_wraps_reads_back_and_reaches_the_scene() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::at(13.5, 150.0));
    port.set_hour("example.a", 25.5).unwrap();
    assert_eq!(port.get("example.a").unwrap().hour, 1.5, "read back at once");
    let mut scene = Recorder::at(13.5, 150.0);
    apply(&bridge, &mut scene);
    let clock = scene.clock.unwrap();
    assert_eq!((clock.hour, clock.seconds_per_hour), (1.5, 150.0), "the rate is kept");
}

#[test]
fn set_day_length_clamps_a_rate_and_zero_holds_the_hour() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::at(9.0, 150.0));
    port.set_day_length("example.a", 2.0).unwrap();
    let mut scene = Recorder::at(9.0, 150.0);
    apply(&bridge, &mut scene);
    assert_eq!(scene.clock.unwrap().seconds_per_hour, 10.0, "the day clock's floor");

    port.set_day_length("example.a", 0.0).unwrap();
    let clock = port.get("example.a").unwrap();
    assert_eq!((clock.frozen, clock.seconds_per_hour), (1, 0.0), "zero stops the clock, and reads as stopped");
    apply(&bridge, &mut scene);
    assert!(scene.held, "the scene holds the hour");
}

#[test]
fn freeze_holds_and_releases_the_hour() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::at(9.0, 150.0));
    port.freeze("example.a", true).unwrap();
    let mut scene = Recorder::at(9.0, 150.0);
    apply(&bridge, &mut scene);
    assert!(scene.held);
    port.freeze("example.a", false).unwrap();
    apply(&bridge, &mut scene);
    assert!(!scene.held);
    assert_eq!(port.get("example.a").unwrap().frozen, 0);
}

#[test]
fn a_change_with_no_scene_clock_is_dropped() {
    let mut scene = Recorder::without_clock();
    time::apply(vec![time::ClockRequest::SetHour(3.0)], &mut scene);
    assert!(scene.clock.is_none(), "a scene without a clock is not given one");
}
