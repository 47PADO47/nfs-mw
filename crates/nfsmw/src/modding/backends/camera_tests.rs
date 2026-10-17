//! `bb.camera` on the game side: the readout is the camera the bridge read at the sync, the views map to the game's modes and
//! back, and a view change is queued for the scene.

use std::rc::Rc;

use blackbox_modding::abi::BB_ERR_NOT_READY;
use blackbox_modding::host::{CameraBackend, CameraKind};

use super::bridge::Bridge;
use super::camera::{CameraPort, kind_of, mode_of};
use super::support::Recorder;
use super::world::apply;
use crate::settings::CameraMode;

fn port() -> (Rc<Bridge>, CameraPort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), CameraPort(bridge))
}

#[test]
fn the_camera_is_unavailable_where_the_scene_has_none() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::without_clock());
    assert_eq!(port.get("example.a"), Err(BB_ERR_NOT_READY));
    assert_eq!(port.set_mode("example.a", CameraKind::Hood), Err(BB_ERR_NOT_READY));
}

#[test]
fn get_reports_the_camera_read_at_the_sync() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::driving([10.0, 2.0, 1.0], 0.0));
    let camera = port.get("example.a").expect("the scene has a camera");
    assert_eq!(camera.kind, CameraKind::Chase);
    assert_eq!(camera.position, [4.0, 2.0, 3.0]);
    assert_eq!((camera.forward, camera.fov), ([1.0, 0.0, 0.0], 70.0));
}

#[test]
fn every_view_maps_to_the_game_mode_and_back() {
    let kinds = [
        CameraKind::Chase,
        CameraKind::DynamicFov,
        CameraKind::Bumper,
        CameraKind::Hood,
        CameraKind::Close,
        CameraKind::Far,
        CameraKind::SuperFar,
        CameraKind::Drift,
        CameraKind::Pursuit,
    ];
    for kind in kinds {
        assert_eq!(kind_of(mode_of(kind)), kind);
    }
    assert_eq!(mode_of(CameraKind::Hood), CameraMode::Hood);
}

#[test]
fn a_view_change_is_queued_and_reaches_the_scene() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::driving([0.0; 3], 0.0));
    port.set_mode("example.a", CameraKind::Hood).expect("the scene has a camera");
    let requests = std::mem::take(&mut *bridge.world_requests.borrow_mut());
    let mut scene = Recorder::driving([0.0; 3], 0.0);
    apply(requests, &mut scene);
    assert_eq!(scene.camera_mode, Some(CameraMode::Hood));
}
