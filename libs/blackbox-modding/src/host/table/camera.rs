//! `bb.camera` on the host side: the backend the game implements, and the trampolines behind `BbCamera`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::camera_view::{ViewHandlers, ViewSpec, read_view};
use super::slot::{Attached, Slot, guard, slot_of, write_prefixed};
use crate::abi::{
    BB_CAMERA_BUMPER, BB_CAMERA_CHASE, BB_CAMERA_CLOSE, BB_CAMERA_DRIFT, BB_CAMERA_DYNAMIC_FOV, BB_CAMERA_EXTRA,
    BB_CAMERA_FAR, BB_CAMERA_HOOD, BB_CAMERA_PURSUIT, BB_CAMERA_SUPER_FAR, BB_ERR_ARG, BbCamera, BbCameraState,
    BbCameraView, BbIface, BbStatus, BbVec3, IFACE_CAMERA, IFACE_CAMERA_VERSION,
};

/// A gameplay view. The numbers are the `BB_CAMERA_*` ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraKind {
    Chase,
    DynamicFov,
    Bumper,
    Hood,
    Close,
    Far,
    SuperFar,
    Drift,
    Pursuit,
    /// One of the extra views mods registered (`bb.camera` 1.1). Only the game reports it; a mod cannot ask for it
    /// with `set_mode`, it uses `select_view`.
    Extra,
}

impl CameraKind {
    /// The view a `BB_CAMERA_*` number names. `None` for an unknown number.
    pub const fn from_raw(raw: u32) -> Option<Self> {
        Some(match raw {
            BB_CAMERA_CHASE => CameraKind::Chase,
            BB_CAMERA_DYNAMIC_FOV => CameraKind::DynamicFov,
            BB_CAMERA_BUMPER => CameraKind::Bumper,
            BB_CAMERA_HOOD => CameraKind::Hood,
            BB_CAMERA_CLOSE => CameraKind::Close,
            BB_CAMERA_FAR => CameraKind::Far,
            BB_CAMERA_SUPER_FAR => CameraKind::SuperFar,
            BB_CAMERA_DRIFT => CameraKind::Drift,
            BB_CAMERA_PURSUIT => CameraKind::Pursuit,
            BB_CAMERA_EXTRA => CameraKind::Extra,
            _ => return None,
        })
    }

    /// The `BB_CAMERA_*` number of this view.
    pub const fn raw(self) -> u32 {
        match self {
            CameraKind::Chase => BB_CAMERA_CHASE,
            CameraKind::DynamicFov => BB_CAMERA_DYNAMIC_FOV,
            CameraKind::Bumper => BB_CAMERA_BUMPER,
            CameraKind::Hood => BB_CAMERA_HOOD,
            CameraKind::Close => BB_CAMERA_CLOSE,
            CameraKind::Far => BB_CAMERA_FAR,
            CameraKind::SuperFar => BB_CAMERA_SUPER_FAR,
            CameraKind::Drift => BB_CAMERA_DRIFT,
            CameraKind::Pursuit => BB_CAMERA_PURSUIT,
            CameraKind::Extra => BB_CAMERA_EXTRA,
        }
    }
}

/// The camera as the game reports it, in render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    pub kind: CameraKind,
    pub position: [f32; 3],
    /// The direction it looks in, unit length.
    pub forward: [f32; 3],
    /// Vertical field of view, degrees.
    pub fov: f32,
}

/// What the game implements for `bb.camera`. `owner` is the id of the mod that made the call.
pub trait CameraBackend {
    /// The camera now.
    fn get(&self, owner: &str) -> BackendResult<CameraState>;
    /// Switch to `kind`. The game answers `BB_ERR_ARG` for a view it does not have.
    fn set_mode(&self, owner: &str, kind: CameraKind) -> BackendResult<()>;
    /// Add an extra view the player can choose (1.1). The handlers stay valid until `unregister_view` or until the mod
    /// unloads. A view id the mod already registered answers `BB_ERR_EXISTS`. Returns the registration's handle.
    fn register_view(&self, owner: &str, spec: &ViewSpec<'_>, handlers: ViewHandlers) -> BackendResult<u64>;
    /// Take back one extra view.
    fn unregister_view(&self, owner: &str, handle: u64) -> BackendResult<()>;
    /// Make the extra view the player's view. `BB_ERR_ARG` for a handle the mod does not hold; a view that does not fit the car is
    /// drawn as the chase camera.
    fn select_view(&self, owner: &str, handle: u64) -> BackendResult<()>;
}

/// Build the `bb.camera` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn CameraBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbCamera {
        head: BbIface {
            size: size_of::<BbCamera>() as u32,
            major: IFACE_CAMERA_VERSION.0,
            minor: IFACE_CAMERA_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        get: Some(get),
        set_mode: Some(set_mode),
        register_view: Some(register_view),
        unregister_view: Some(unregister_view),
        select_view: Some(select_view),
    });
    let kept = slot.clone();
    let release = Box::new(move || {
        kept.release_with(|backend, owner, handle| {
            let _ = backend.unregister_view(owner, handle);
        })
    });
    Attached::new(IFACE_CAMERA, IFACE_CAMERA_VERSION, table, release)
}

/// `get`: write the camera to `out`, as far as the mod's size word allows.
unsafe extern "C" fn get(ctx: *mut c_void, out: *mut BbCameraState) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CameraBackend>(ctx)? };
        let camera = slot.backend.get(&slot.owner)?;
        let wire = BbCameraState {
            size: size_of::<BbCameraState>() as u32,
            mode: camera.kind.raw(),
            position: vec3(camera.position),
            forward: vec3(camera.forward),
            fov: camera.fov,
        };
        // SAFETY: `out` is the mod's struct; `write_prefixed` checks its size word before it writes.
        unsafe { write_prefixed(out, &wire) }
    })
}

/// `set_mode`: switch to the view `mode` names.
unsafe extern "C" fn set_mode(ctx: *mut c_void, mode: u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CameraBackend>(ctx)? };
        let kind = CameraKind::from_raw(mode).filter(|kind| *kind != CameraKind::Extra).ok_or(BB_ERR_ARG)?;
        slot.backend.set_mode(&slot.owner, kind)
    })
}

/// `register_view`: add an extra view; the handle goes to `out_id`.
unsafe extern "C" fn register_view(ctx: *mut c_void, view: *const BbCameraView, out_id: *mut u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CameraBackend>(ctx)? };
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        // SAFETY: the mod's view is valid for the call, its callbacks until the view is unregistered (see `read_view`).
        let (spec, handlers) = unsafe { read_view(view) }?;
        let handle = slot.backend.register_view(&slot.owner, &spec, handlers)?;
        slot.remember(handle);
        // SAFETY: `out_id` is non-null (checked above) and valid for one write.
        unsafe { out_id.write(handle) };
        Ok(())
    })
}

/// `unregister_view`: take back one view.
unsafe extern "C" fn unregister_view(ctx: *mut c_void, id: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CameraBackend>(ctx)? };
        slot.backend.unregister_view(&slot.owner, id)?;
        slot.forget(id);
        Ok(())
    })
}

/// `select_view`: look through a registered view.
unsafe extern "C" fn select_view(ctx: *mut c_void, id: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CameraBackend>(ctx)? };
        slot.backend.select_view(&slot.owner, id)
    })
}

/// A render-space point in the ABI's plain struct.
fn vec3(value: [f32; 3]) -> BbVec3 {
    BbVec3 { x: value[0], y: value[1], z: value[2] }
}
