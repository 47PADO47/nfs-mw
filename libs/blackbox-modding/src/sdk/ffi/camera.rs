//! `bb.camera` on the mod side: read the table, read the camera, choose a view, and register extra views.

use core::cell::RefCell;
use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::null_mut;

use super::{HostTable, finish, read_or, status_result};
use crate::abi::{
    BB_CAMERA_INPUT_RESET, BB_CAMERA_POSE_HIDE_CAR, BB_ERR_ARG, BB_ERR_PANIC, BB_OK, BbCamera, BbCameraGetFn,
    BbCameraInput, BbCameraPose, BbCameraRegisterViewFn, BbCameraSelectViewFn, BbCameraSetModeFn, BbCameraState,
    BbCameraUnregisterViewFn, BbCameraView, BbStatus, BbStr,
};
use crate::sdk::ctx::Keep;
use crate::sdk::{CameraInput, CameraPose, CameraView, CameraViewId, Error, Interface, ModResult, guard};

/// The function pointers of one `bb.camera` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct CameraFns {
    ctx: *mut c_void,
    get: Option<BbCameraGetFn>,
    set_mode: Option<BbCameraSetModeFn>,
    register_view: Option<BbCameraRegisterViewFn>,
    unregister_view: Option<BbCameraUnregisterViewFn>,
    select_view: Option<BbCameraSelectViewFn>,
}

/// Read the `bb.camera` table the host offered. Fields the host's minor version lacks read as absent.
pub(crate) fn read(iface: Interface) -> CameraFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        CameraFns {
            ctx: read_or(&iface, offset_of!(BbCamera, ctx), null_mut()),
            get: read_or(&iface, offset_of!(BbCamera, get), None),
            set_mode: read_or(&iface, offset_of!(BbCamera, set_mode), None),
            register_view: read_or(&iface, offset_of!(BbCamera, register_view), None),
            unregister_view: read_or(&iface, offset_of!(BbCamera, unregister_view), None),
            select_view: read_or(&iface, offset_of!(BbCamera, select_view), None),
        }
    }
}

/// The camera now, as the host wrote it (the mode is still the raw `BB_CAMERA_*` number).
pub(crate) fn get(fns: &CameraFns) -> ModResult<BbCameraState> {
    let get = fns.get.ok_or(Error::Unsupported)?;
    let mut raw = BbCameraState { size: size_of::<BbCameraState>() as u32, ..BbCameraState::default() };
    // SAFETY: `raw` is valid for one write of its whole layout, and its size word says so.
    status_result(unsafe { get(fns.ctx, &mut raw) })?;
    Ok(raw)
}

/// Switch to the view `mode` names.
pub(crate) fn set_mode(fns: &CameraFns, mode: u32) -> ModResult<()> {
    let set_mode = fns.set_mode.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded; a scalar argument needs no other check.
    status_result(unsafe { set_mode(fns.ctx, mode) })
}

/// A registered view: the mod's object, and what the host's callbacks need to call it.
struct Entry {
    host: HostTable,
    view: RefCell<Box<dyn CameraView>>,
}

/// Register an extra view. The object is kept in `keep` until the mod unloads.
pub(crate) fn register_view(
    fns: &CameraFns,
    host: HostTable,
    keep: &Keep,
    id: &str,
    name: &str,
    view: Box<dyn CameraView>,
) -> ModResult<CameraViewId> {
    let register = fns.register_view.ok_or(Error::Unsupported)?;
    let entry = Box::new(Entry { host, view: RefCell::new(view) });
    let wire = BbCameraView {
        size: size_of::<BbCameraView>() as u32,
        flags: 0,
        id: BbStr::new(id),
        name: BbStr::new(name),
        user: (&*entry as *const Entry).cast_mut().cast(),
        available: Some(available_trampoline),
        update: Some(update_trampoline),
    };
    let mut handle = 0_u64;
    // SAFETY: the host calls the trampolines with `user` only while the registration exists. `entry` moves into `keep`
    // below, which lives until the mod's `destroy`; the host removes the registration when the mod unloads. `wire` and
    // its strings are valid for this call.
    let status = unsafe { register(fns.ctx, &wire, &mut handle) };
    status_result(status)?;
    keep.borrow_mut().push(entry);
    Ok(CameraViewId::from_raw(handle))
}

/// Take back one extra view.
pub(crate) fn unregister_view(fns: &CameraFns, id: CameraViewId) -> ModResult<()> {
    let unregister = fns.unregister_view.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { unregister(fns.ctx, id.raw()) })
}

/// Look through an extra view.
pub(crate) fn select_view(fns: &CameraFns, id: CameraViewId) -> ModResult<()> {
    let select = fns.select_view.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { select(fns.ctx, id.raw()) })
}

/// The host's question whether the view fits a car: runs the object's `available`.
unsafe extern "C" fn available_trampoline(user: *mut c_void, car: BbStr) -> u32 {
    if user.is_null() {
        return 0;
    }
    // SAFETY: `user` is the `Entry` that `register_view` kept alive.
    let entry = unsafe { &*user.cast::<Entry>() };
    // SAFETY: the host passes a string valid for this call.
    let Some(car) = (unsafe { car.as_str() }) else {
        return 0;
    };
    let Ok(mut view) = entry.view.try_borrow_mut() else {
        return 0;
    };
    let result = guard::run(|| Ok(view.available(car)));
    u32::from(finish(entry.host, "camera view available", result).unwrap_or(false))
}

/// The host's per-frame call: runs the object's `update` and writes the pose it answers.
unsafe extern "C" fn update_trampoline(
    user: *mut c_void,
    input: *const BbCameraInput,
    out: *mut BbCameraPose,
) -> BbStatus {
    if user.is_null() || input.is_null() || out.is_null() {
        return BB_ERR_ARG;
    }
    // SAFETY: `user` is the `Entry` that `register_view` kept alive.
    let entry = unsafe { &*user.cast::<Entry>() };
    // SAFETY: the host's input is valid for the call; a shorter struct than ours is refused by its size word.
    let raw = unsafe { *input };
    if (raw.size as usize) < size_of::<BbCameraInput>() {
        return BB_ERR_ARG;
    }
    // SAFETY: the host passes a string valid for this call.
    let Some(car) = (unsafe { raw.car.as_str() }) else {
        return BB_ERR_ARG;
    };
    let Ok(mut view) = entry.view.try_borrow_mut() else {
        return BB_ERR_ARG;
    };
    let input = CameraInput {
        reset: raw.flags & BB_CAMERA_INPUT_RESET != 0,
        dt: raw.dt,
        speed: raw.speed,
        steer: raw.steer,
        throttle: raw.throttle,
        brake: raw.brake,
        impact: raw.impact,
        grounded: raw.grounded != 0,
        position: raw.position.to_array(),
        forward: raw.forward.to_array(),
        up: raw.up.to_array(),
        velocity: raw.velocity.to_array(),
        angular_velocity: raw.angular_velocity.to_array(),
        car,
    };
    let result = guard::run(|| Ok(view.update(&input)));
    match finish(entry.host, "camera view update", result) {
        Ok(Some(pose)) => {
            let wire = BbCameraPose {
                size: size_of::<BbCameraPose>() as u32,
                flags: if pose.hide_car { BB_CAMERA_POSE_HIDE_CAR } else { 0 },
                position: vec3(pose.position),
                forward: vec3(pose.forward),
                up: vec3(pose.up),
                fov: pose.fov,
                pad: 0,
            };
            // SAFETY: `out` is the host's struct, valid for one write of the layout both sides share.
            unsafe { out.write(wire) };
            BB_OK
        }
        Ok(None) => BB_ERR_ARG,
        Err(_) => BB_ERR_PANIC,
    }
}

fn vec3(value: [f32; 3]) -> crate::abi::BbVec3 {
    crate::abi::BbVec3 { x: value[0], y: value[1], z: value[2] }
}
