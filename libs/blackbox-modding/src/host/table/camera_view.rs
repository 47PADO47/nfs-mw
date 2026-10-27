//! The extra views of `bb.camera` 1.1 on the host side: what a mod registers, the frame input and the pose in plain Rust,
//! and the closures that wrap the mod's C callbacks so the game never touches a function pointer.

use core::ffi::c_void;
use core::mem::size_of;

use super::slot::name_of;
use crate::abi::{
    BB_CAMERA_INPUT_RESET, BB_CAMERA_POSE_HIDE_CAR, BB_ERR_ARG, BB_OK, BbCameraAvailableFn, BbCameraInput,
    BbCameraPose, BbCameraUpdateFn, BbCameraView, BbStatus, BbStr, BbVec3, MAX_NAME_BYTES,
};

/// An extra view a mod registers: its id (unique in the mod) and the name the camera setting shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewSpec<'a> {
    pub id: &'a str,
    pub name: &'a str,
}

/// The car and the frame an extra view is stepped with, in render space (x forward, y left, z up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewInput<'a> {
    /// The view was just selected or the car was moved: place the camera at rest.
    pub reset: bool,
    pub dt: f32,
    /// Speed along the car, m/s (negative in reverse).
    pub speed: f32,
    /// -1 to 1, positive to the left.
    pub steer: f32,
    pub throttle: f32,
    pub brake: f32,
    /// Collision strength this frame, 0 to 1.
    pub impact: f32,
    pub grounded: bool,
    pub position: [f32; 3],
    pub forward: [f32; 3],
    pub up: [f32; 3],
    pub velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    /// The id of the car the player drives.
    pub car: &'a str,
}

/// The camera an extra view answers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewPose {
    pub position: [f32; 3],
    /// Unit length.
    pub forward: [f32; 3],
    /// Unit length.
    pub up: [f32; 3],
    /// Vertical field of view, degrees.
    pub fov: f32,
    /// The player's car is not drawn (a camera inside or against the car).
    pub hide_car: bool,
}

/// The two calls a registered view answers, as closures over the mod's callbacks.
pub struct ViewHandlers {
    /// Whether the view can be used with the car of that id.
    pub available: Box<dyn Fn(&str) -> bool>,
    /// The pose for this frame, `None` when the mod declined it.
    pub update: Box<dyn Fn(&ViewInput<'_>) -> Option<ViewPose>>,
}

fn vec3(value: [f32; 3]) -> BbVec3 {
    BbVec3 { x: value[0], y: value[1], z: value[2] }
}

/// Read what the mod registered: the strings, and the handlers over its callbacks.
///
/// # Safety
/// `view` must be valid for reads of the bytes its own `size` field declares, its strings valid for the call, and `user`
/// and the callbacks valid until the view is unregistered (ADR 0005 section 4).
pub(super) unsafe fn read_view<'a>(view: *const BbCameraView) -> Result<(ViewSpec<'a>, ViewHandlers), BbStatus> {
    if view.is_null() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the view's size word is its first field (see `# Safety`).
    let size = unsafe { view.cast::<u32>().read() };
    if (size as usize) < size_of::<BbCameraView>() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the size covers the struct, so the read is in bounds.
    let view = unsafe { view.read() };
    let update = view.update.ok_or(BB_ERR_ARG)?;
    // SAFETY: the mod passes strings valid for the call.
    let id = unsafe { name_of(view.id) }?;
    // SAFETY: as above.
    let name = unsafe { super::slot::text_within(view.name, MAX_NAME_BYTES) }?;
    let user = UserPtr(view.user);
    Ok((
        ViewSpec { id, name },
        ViewHandlers { available: available_closure(user, view.available), update: update_closure(user, update) },
    ))
}

/// The mod's `user` pointer, carried into the closures. The mod keeps it valid until the view is unregistered.
#[derive(Clone, Copy)]
struct UserPtr(*mut c_void);

fn available_closure(user: UserPtr, available: Option<BbCameraAvailableFn>) -> Box<dyn Fn(&str) -> bool> {
    Box::new(move |car: &str| {
        let Some(available) = available else { return true };
        // SAFETY: the mod keeps `user` valid until the view is unregistered; the string lives for this call.
        unsafe { available(user.0, BbStr::new(car)) != 0 }
    })
}

fn update_closure(user: UserPtr, update: BbCameraUpdateFn) -> Box<dyn Fn(&ViewInput<'_>) -> Option<ViewPose>> {
    Box::new(move |input: &ViewInput<'_>| {
        let wire = BbCameraInput {
            size: size_of::<BbCameraInput>() as u32,
            flags: if input.reset { BB_CAMERA_INPUT_RESET } else { 0 },
            dt: input.dt,
            speed: input.speed,
            steer: input.steer,
            throttle: input.throttle,
            brake: input.brake,
            impact: input.impact,
            grounded: u32::from(input.grounded),
            pad: 0,
            position: vec3(input.position),
            forward: vec3(input.forward),
            up: vec3(input.up),
            velocity: vec3(input.velocity),
            angular_velocity: vec3(input.angular_velocity),
            car: BbStr::new(input.car),
        };
        let mut pose = BbCameraPose::default();
        // SAFETY: the mod keeps `user` valid until the view is unregistered; `wire` and `pose` are valid for the call.
        let status = unsafe { update(user.0, &wire, &mut pose) };
        if status != BB_OK || (pose.size as usize) < size_of::<BbCameraPose>() {
            return None;
        }
        Some(ViewPose {
            position: pose.position.to_array(),
            forward: pose.forward.to_array(),
            up: pose.up.to_array(),
            fov: pose.fov,
            hide_car: pose.flags & BB_CAMERA_POSE_HIDE_CAR != 0,
        })
    })
}
