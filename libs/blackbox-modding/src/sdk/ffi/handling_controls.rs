//! `bb.handling` 1.2 on the mod side: register a Rust closure as the controls hook. The table reading and the shared helpers are in
//! `handling.rs`.

use core::cell::RefCell;
use core::ffi::c_void;
use core::mem::size_of;

use super::handling::{HandlingFns, HookEntry, read_query, write_result};
use super::{HostTable, finish, status_result};
use crate::abi::{
    BB_CONTROLS_FLAG_STEER, BB_CONTROLS_HANDLED, BB_CONTROLS_NOT_HANDLED, BB_ERR_ARG, BbControlsHook, BbControlsQuery,
    BbControlsResult, BbStatus,
};
use crate::sdk::ctx::Keep;
use crate::sdk::{ControlsQuery, ControlsResult, Ctx, Error, HandlingHookId, ModResult, Role, guard};

/// Register a closure as the controls hook. The closure is kept in `keep` until the mod unloads.
pub(crate) fn register_controls<F>(
    fns: &HandlingFns,
    host: HostTable,
    keep: &Keep,
    handler: F,
) -> ModResult<HandlingHookId>
where
    F: FnMut(&mut Ctx, &ControlsQuery) -> Option<ControlsResult> + 'static,
{
    let register = fns.register_controls.ok_or(Error::Unsupported)?;
    let entry = Box::new(HookEntry { host, keep: keep.clone(), handler: RefCell::new(handler) });
    let user: *mut c_void = (&*entry as *const HookEntry<F>).cast_mut().cast();
    let hook = BbControlsHook {
        size: size_of::<BbControlsHook>() as u32,
        flags: 0,
        user,
        callback: Some(controls_trampoline::<F>),
        reserved: [0; 4],
    };
    let mut id = 0_u64;
    // SAFETY: the host calls `controls_trampoline::<F>` with `user` only while the hook is registered. `entry` moves into `keep`
    // below, which lives until the mod's `destroy`; the host retires the hook before the mod unloads. `hook` and `id` are valid
    // for the call.
    let status = unsafe { register(fns.ctx, &hook, &mut id) };
    status_result(status)?;
    keep.borrow_mut().push(entry);
    Ok(HandlingHookId::from_raw(id))
}

/// Remove the controls hook this mod registered.
pub(crate) fn unregister_controls(fns: &HandlingFns, id: HandlingHookId) -> ModResult<()> {
    let unregister = fns.unregister_controls.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { unregister(fns.ctx, id.raw()) })
}

fn mod_query(wire: &BbControlsQuery) -> ControlsQuery {
    ControlsQuery {
        role: Role::from_raw(wire.role),
        vehicle_revision: wire.vehicle_revision,
        dt: wire.dt,
        throttle: wire.throttle,
        brake: wire.brake,
        handbrake: wire.handbrake,
        steer: wire.steer,
        forward_speed: wire.forward_speed,
        yaw_rate: wire.yaw_rate,
        lateral_accel: wire.lateral_accel,
        longitudinal_accel: wire.longitudinal_accel,
        wheel_loaded: std::array::from_fn(|i| wire.wheel_loaded_mask & (1 << i) != 0),
        slip_ratio: wire.slip_ratio,
        slip_angle: wire.slip_angle,
    }
}

fn wire_result(result: &ControlsResult) -> BbControlsResult {
    BbControlsResult {
        size: size_of::<BbControlsResult>() as u32,
        flags: if result.steer_target_rad.is_some() { BB_CONTROLS_FLAG_STEER } else { 0 },
        throttle: result.throttle,
        brake: result.brake,
        steer_target_rad: result.steer_target_rad.unwrap_or(0.0),
        reserved: [0; 8],
    }
}

/// The callback the host calls each tick: runs the closure in `user`.
unsafe extern "C" fn controls_trampoline<F>(
    user: *mut c_void,
    query: *const BbControlsQuery,
    result: *mut BbControlsResult,
) -> BbStatus
where
    F: FnMut(&mut Ctx, &ControlsQuery) -> Option<ControlsResult> + 'static,
{
    if user.is_null() || query.is_null() || result.is_null() {
        return BB_ERR_ARG;
    }
    // SAFETY: `user` is the `HookEntry<F>` that `register_controls` kept alive (see there).
    let entry = unsafe { &*user.cast::<HookEntry<F>>() };
    // A call that arrives while the closure runs (a host call from inside it that steps the physics) is declined.
    let Ok(mut handler) = entry.handler.try_borrow_mut() else {
        return BB_CONTROLS_NOT_HANDLED;
    };
    // SAFETY: the host passes a query valid for the bytes its size word declares.
    let Some(wire) = (unsafe { read_query::<BbControlsQuery>(query) }) else {
        return BB_ERR_ARG;
    };
    let question = mod_query(&wire);
    let mut ctx = Ctx::new(entry.host, entry.keep.clone());
    let outcome = guard::run(|| Ok((*handler)(&mut ctx, &question)));
    match finish(entry.host, "controls hook", outcome) {
        Ok(Some(answer)) => {
            // SAFETY: the host passes a result valid for the bytes its size word declares.
            unsafe { write_result(result, &wire_result(&answer)) };
            BB_CONTROLS_HANDLED
        }
        Ok(None) => BB_CONTROLS_NOT_HANDLED,
        Err(error) => error.code(),
    }
}
