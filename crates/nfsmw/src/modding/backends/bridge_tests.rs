use std::rc::Rc;

use blackbox_modding::host::{BlipSpec, MapBackend, TimeBackend};

use super::bridge::Bridge;
use super::map::MapPort;
use super::support::Recorder;
use super::time::TimePort;
use crate::devtools::Console;
use crate::hud::COP;

#[test]
fn a_frame_copies_the_clock_in_and_hands_the_changes_back_to_the_scene() {
    let bridge = Rc::new(Bridge::default());
    let time = TimePort(bridge.clone());
    let map = MapPort(bridge.clone());

    bridge.sync(&Recorder::at(9.0, 150.0));
    assert_eq!(time.get("example.a").unwrap().hour, 9.0, "the mods read the scene's clock");

    time.set_hour("example.a", 20.0).unwrap();
    map.add("example.a", &BlipSpec { style: COP, position: [4.0, -1.0, 0.0], active: true }).unwrap();

    let mut scene = Recorder::at(9.0, 150.0);
    let mut console = Console::default();
    bridge.apply(&mut scene, &mut console);
    assert_eq!(scene.clock.map(|clock| clock.hour), Some(20.0));
    assert_eq!(scene.blips.len(), 1, "the minimap gets the mod blips");
    assert!(bridge.clock_requests.borrow().is_empty(), "the requests are taken once");

    let mut next = Recorder::at(20.0, 150.0);
    bridge.apply(&mut next, &mut console);
    assert_eq!(next.blips.len(), 1, "the blips are given to the scene every frame, even a new one");
}

#[test]
fn a_scene_without_a_clock_leaves_the_mods_with_no_time() {
    let bridge = Rc::new(Bridge::default());
    bridge.sync(&Recorder::without_clock());
    assert!(bridge.clock.get().is_none());
    let mut scene = Recorder::without_clock();
    bridge.apply(&mut scene, &mut Console::default());
    assert!(scene.clock.is_none());
}
