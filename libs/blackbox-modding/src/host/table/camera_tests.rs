//! `bb.camera` called directly with a fake backend: the camera is written up to the mod's size, a view is chosen by its
//! number, and an unknown number is refused before the backend.

use crate::host::test_support::callback_on_main;
use core::ffi::c_void;
use std::cell::RefCell;
use std::mem::size_of;
use std::rc::Rc;

use super::camera::{CameraBackend, CameraKind, CameraState};
use super::camera_view::{ViewHandlers, ViewInput, ViewSpec};
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
    views: RefCell<Vec<(String, String, ViewHandlers)>>,
    unregistered: RefCell<Vec<u64>>,
    selected: RefCell<Vec<u64>>,
}

impl CameraBackend for FakeCamera {
    fn get(&self, _owner: &str) -> BackendResult<CameraState> {
        Ok(CameraState { kind: CameraKind::Bumper, position: [-6.0, 0.0, 1.2], forward: [1.0, 0.0, 0.0], fov: 70.0 })
    }

    fn set_mode(&self, _owner: &str, kind: CameraKind) -> BackendResult<()> {
        self.modes.borrow_mut().push(kind);
        Ok(())
    }

    fn register_view(&self, _owner: &str, spec: &ViewSpec<'_>, handlers: ViewHandlers) -> BackendResult<u64> {
        let mut views = self.views.borrow_mut();
        if views.iter().any(|(id, _, _)| id == spec.id) {
            return Err(BB_ERR_EXISTS);
        }
        views.push((spec.id.to_owned(), spec.name.to_owned(), handlers));
        Ok(views.len() as u64)
    }

    fn unregister_view(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        self.unregistered.borrow_mut().push(handle);
        Ok(())
    }

    fn select_view(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        self.selected.borrow_mut().push(handle);
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
    assert_eq!(unsafe { set_mode(table.ctx, BB_CAMERA_EXTRA) }, BB_ERR_ARG, "extra views are chosen with select_view");
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
        BB_CAMERA_EXTRA,
    ];
    for number in numbers {
        assert_eq!(CameraKind::from_raw(number).map(CameraKind::raw), Some(number));
    }
    assert_eq!(CameraKind::from_raw(9), None);
}

/// A mod's view: answers the car it is built for, and puts the camera 5 m behind the car's position.
unsafe extern "C" fn behind_available(user: *mut c_void, car: BbStr) -> u32 {
    assert_eq!(user as usize, 7);
    u32::from(unsafe { car.as_str() } == Some("CARRERAGT"))
}

unsafe extern "C" fn behind_update(user: *mut c_void, input: *const BbCameraInput, out: *mut BbCameraPose) -> BbStatus {
    assert_eq!(user as usize, 7);
    let input = unsafe { &*input };
    if input.dt < 0.0 {
        return BB_ERR_ARG;
    }
    unsafe {
        out.write(BbCameraPose {
            size: size_of::<BbCameraPose>() as u32,
            position: BbVec3 { x: input.position.x - 5.0, y: input.position.y, z: input.position.z + 1.5 },
            forward: input.forward,
            up: input.up,
            fov: 60.0 + input.speed,
            flags: BB_CAMERA_POSE_HIDE_CAR,
            ..BbCameraPose::default()
        });
    }
    BB_OK
}

fn view(id: &'static str) -> BbCameraView {
    BbCameraView {
        size: size_of::<BbCameraView>() as u32,
        flags: 0,
        id: BbStr::new(id),
        name: BbStr::new("Behind"),
        user: 7 as *mut c_void,
        available: Some(behind_available),
        update: Some(behind_update),
    }
}

fn input(dt: f32) -> ViewInput<'static> {
    ViewInput {
        reset: false,
        dt,
        speed: 10.0,
        steer: 0.0,
        throttle: 1.0,
        brake: 0.0,
        impact: 0.0,
        grounded: true,
        position: [100.0, 20.0, 1.0],
        forward: [1.0, 0.0, 0.0],
        up: [0.0, 0.0, 1.0],
        velocity: [10.0, 0.0, 0.0],
        angular_velocity: [0.0; 3],
        car: "CARRERAGT",
    }
}

#[test]
fn a_registered_view_is_called_through_its_handlers() {
    let _scope = callback_on_main();
    let fake = Rc::new(FakeCamera::default());
    let attached = attach_camera(fake.clone(), "example.a");
    let table = table_of(&attached);
    assert!(table.register_view.is_some() && table.unregister_view.is_some() && table.select_view.is_some());
    let register = table.register_view.unwrap();
    let mut handle = 0_u64;
    assert_eq!(unsafe { register(table.ctx, &view("behind"), &mut handle) }, BB_OK);
    assert_eq!(handle, 1);
    assert_eq!(unsafe { register(table.ctx, &view("behind"), &mut handle) }, BB_ERR_EXISTS);

    let views = fake.views.borrow();
    let (id, name, handlers) = &views[0];
    assert_eq!((id.as_str(), name.as_str()), ("behind", "Behind"));
    assert!((handlers.available)("CARRERAGT"));
    assert!(!(handlers.available)("OTHER"));
    let pose = (handlers.update)(&input(0.016)).expect("the mod answered");
    assert_eq!(pose.position, [95.0, 20.0, 2.5]);
    assert_eq!(pose.forward, [1.0, 0.0, 0.0]);
    assert_eq!(pose.fov, 70.0);
    assert!(pose.hide_car);
    assert_eq!((handlers.update)(&input(-1.0)), None, "a status other than BB_OK keeps the host's camera");
}

#[test]
fn a_view_without_update_or_with_a_short_struct_is_refused() {
    let _scope = callback_on_main();
    let attached = attach_camera(Rc::new(FakeCamera::default()), "example.a");
    let table = table_of(&attached);
    let register = table.register_view.unwrap();
    let mut handle = 0_u64;
    let mut no_update = view("a");
    no_update.update = None;
    assert_eq!(unsafe { register(table.ctx, &no_update, &mut handle) }, BB_ERR_ARG);
    let mut short = view("b");
    short.size = 8;
    assert_eq!(unsafe { register(table.ctx, &short, &mut handle) }, BB_ERR_ARG);
    assert_eq!(unsafe { register(table.ctx, core::ptr::null(), &mut handle) }, BB_ERR_ARG);
    assert_eq!(unsafe { register(table.ctx, &view("c"), core::ptr::null_mut()) }, BB_ERR_ARG);
}

#[test]
fn views_are_selected_and_taken_back_with_the_mod() {
    let _scope = callback_on_main();
    let fake = Rc::new(FakeCamera::default());
    let attached = attach_camera(fake.clone(), "example.a");
    let table = table_of(&attached);
    let mut handle = 0_u64;
    assert_eq!(unsafe { table.register_view.unwrap()(table.ctx, &view("behind"), &mut handle) }, BB_OK);
    assert_eq!(unsafe { table.select_view.unwrap()(table.ctx, handle) }, BB_OK);
    assert_eq!(*fake.selected.borrow(), vec![1]);
    drop(attached);
    assert_eq!(*fake.unregistered.borrow(), vec![1], "unloading the mod takes its views back");
}
