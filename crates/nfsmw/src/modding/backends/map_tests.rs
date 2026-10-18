use std::rc::Rc;

use blackbox_modding::abi::BB_ERR_NOT_FOUND;
use blackbox_modding::host::{BlipSpec, MapBackend};

use super::bridge::Bridge;
use super::map::{MapPort, minimap_blips};
use crate::hud::{COP, MapBlip, RACE_SPRINT};

fn port() -> (Rc<Bridge>, MapPort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), MapPort(bridge))
}

fn spec(style: &str, x: f32, active: bool) -> BlipSpec<'_> {
    BlipSpec { style, position: [x, 2.0, 0.0], active }
}

#[test]
fn a_blip_takes_a_game_style_and_reaches_the_minimap() {
    let (bridge, port) = port();
    let handle = port.add("example.a", &spec(COP, 5.0, true)).unwrap();
    assert_ne!(handle, 0);
    assert_eq!(minimap_blips(&bridge), vec![MapBlip { style: COP, position: [5.0, 2.0], active: true }]);
}

#[test]
fn a_style_the_game_does_not_have_is_not_found() {
    let (bridge, port) = port();
    assert_eq!(port.add("example.a", &spec("unicorn", 0.0, false)).map(|_| ()), Err(BB_ERR_NOT_FOUND));
    assert!(bridge.blips.borrow().is_empty());
}

#[test]
fn update_moves_an_owned_blip_and_refuses_another_mods() {
    let (bridge, port) = port();
    let handle = port.add("example.a", &spec(COP, 5.0, false)).unwrap();
    port.update("example.a", handle, &spec(RACE_SPRINT, 9.0, true)).unwrap();
    assert_eq!(minimap_blips(&bridge), vec![MapBlip { style: RACE_SPRINT, position: [9.0, 2.0], active: true }]);
    assert_eq!(port.update("example.b", handle, &spec(COP, 0.0, false)), Err(BB_ERR_NOT_FOUND));
}

#[test]
fn remove_drops_the_blip_once() {
    let (bridge, port) = port();
    let handle = port.add("example.a", &spec(COP, 5.0, false)).unwrap();
    assert_eq!(port.remove("example.b", handle), Err(BB_ERR_NOT_FOUND));
    port.remove("example.a", handle).unwrap();
    assert!(minimap_blips(&bridge).is_empty());
    assert_eq!(port.remove("example.a", handle), Err(BB_ERR_NOT_FOUND));
}

#[test]
fn every_blip_gets_its_own_handle() {
    let (_, port) = port();
    let first = port.add("example.a", &spec(COP, 1.0, false)).unwrap();
    let second = port.add("example.a", &spec(COP, 2.0, false)).unwrap();
    assert_ne!(first, second);
    assert!(first != 0 && second != 0, "0 means none");
}
