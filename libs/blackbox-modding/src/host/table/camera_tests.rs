//! `bb.camera` called directly with a fake backend: the camera is written up to the mod's size, a view is chosen by its
//! number, and an unknown number is refused before the backend.

use crate::host::test_support::callback_on_main;
use std::cell::RefCell;
use std::mem::size_of;
use std::rc::Rc;

use super::camera::{CameraBackend, CameraKind, CameraState};
use super::{Attached, attach_camera};
use crate::abi::*;
use crate::host::BackendResult;

fn table_of(attached: &Attached) -> &BbCamera {
    // SAFETY: the attachment owns a `BbCamera` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbCamera>() }
}

#[derive(Default)]
struct FakeCamera {
    modes: RefCell<Vec<CameraKind>>,
}

impl CameraBackend for FakeCamera {
    fn get(&self, _owner: &str) -> BackendResult<CameraState> {
        Ok(CameraState { kind: CameraKind::Bumper, position: [-6.0, 0.0, 1.2], forward: [1.0, 0.0, 0.0], fov: 70.0 })
    }

    fn set_mode(&self, _owner: &str, kind: CameraKind) -> BackendResult<()> {
        self.modes.borrow_mut().push(kind);
        Ok(())
    }
}

#[test]
fn the_camera_is_written_in_full() {
    let _scope = callback_on_main();
    let attached = attach_camera(Rc::new(FakeCamera::default()), "example.a");
    let table = table_of(&attached);
    let mut out = BbCameraState { size: size_of::<BbCameraState>() as u32, ..BbCameraState::default() };
    let get = table.get.expect("get is offered");
    assert_eq!(unsafe { get(table.ctx, &mut out) }, BB_OK);
    assert_eq!(out.mode, BB_CAMERA_BUMPER);
    assert_eq!(out.position.x, -6.0);
    assert_eq!(out.forward.x, 1.0);
    assert_eq!(out.fov, 70.0);
}

#[test]
fn set_mode_takes_a_known_view_and_refuses_the_rest() {
    let _scope = callback_on_main();
    let fake = Rc::new(FakeCamera::default());
    let attached = attach_camera(fake.clone(), "example.a");
    let table = table_of(&attached);
    let set_mode = table.set_mode.expect("set_mode is offered");
    assert_eq!(unsafe { set_mode(table.ctx, BB_CAMERA_HOOD) }, BB_OK);
    assert_eq!(unsafe { set_mode(table.ctx, 42) }, BB_ERR_ARG);
    assert_eq!(*fake.modes.borrow(), vec![CameraKind::Hood]);
}

#[test]
fn every_view_number_round_trips() {
    let _scope = callback_on_main();
    let numbers = [
        BB_CAMERA_CHASE,
        BB_CAMERA_DYNAMIC_FOV,
        BB_CAMERA_BUMPER,
        BB_CAMERA_HOOD,
        BB_CAMERA_CLOSE,
        BB_CAMERA_FAR,
        BB_CAMERA_SUPER_FAR,
        BB_CAMERA_DRIFT,
        BB_CAMERA_PURSUIT,
    ];
    for number in numbers {
        assert_eq!(CameraKind::from_raw(number).map(CameraKind::raw), Some(number));
    }
    assert_eq!(CameraKind::from_raw(9), None);
}
