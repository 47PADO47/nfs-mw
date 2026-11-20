//! `bb.handling` 1.1 on the mod side: register a Rust closure as the suspension hook. The table reading and the shared helpers are
//! in `handling.rs`.

use core::cell::RefCell;
use core::ffi::c_void;
use core::mem::size_of;

use super::handling::{HandlingFns, HookEntry, read_query, write_result};
use super::{HostTable, finish, status_result};
use crate::abi::{
    BB_ERR_ARG, BB_SUSPENSION_HANDLED, BB_SUSPENSION_NOT_HANDLED, BbStatus, BbSuspensionHook, BbSuspensionQuery,
    BbSuspensionResult, BbSuspensionWheel, BbSuspensionWheelResult,
};
use crate::sdk::ctx::Keep;
use crate::sdk::{
    Ctx, Error, HandlingHookId, ModResult, Role, SuspensionQuery, SuspensionResult, SuspensionWheelQuery,
    SuspensionWheelResult, guard,
};

/// Register a closure as the suspension hook. The closure is kept in `keep` until the mod unloads.
pub(crate) fn register_suspension<F>(
    fns: &HandlingFns,
    host: HostTable,
    keep: &Keep,
    handler: F,
) -> ModResult<HandlingHookId>
where
    F: FnMut(&mut Ctx, &SuspensionQuery) -> Option<SuspensionResult> + 'static,
{
    let register = fns.register_suspension.ok_or(Error::Unsupported)?;
    let entry = Box::new(HookEntry { host, keep: keep.clone(), handler: RefCell::new(handler) });
    let user: *mut c_void = (&*entry as *const HookEntry<F>).cast_mut().cast();
    let hook = BbSuspensionHook {
        size: size_of::<BbSuspensionHook>() as u32,
        flags: 0,
        user,
        callback: Some(suspension_trampoline::<F>),
        reserved: [0; 4],
    };
    let mut id = 0_u64;
    // SAFETY: the host calls `suspension_trampoline::<F>` with `user` only while the hook is registered. `entry` moves into `keep`
    // below, which lives until the mod's `destroy`; the host retires the hook before the mod unloads. `hook` and `id` are valid
    // for the call.
    let status = unsafe { register(fns.ctx, &hook, &mut id) };
    status_result(status)?;
    keep.borrow_mut().push(entry);
    Ok(HandlingHookId::from_raw(id))
}

/// Remove the suspension hook this mod registered.
pub(crate) fn unregister_suspension(fns: &HandlingFns, id: HandlingHookId) -> ModResult<()> {
    let unregister = fns.unregister_suspension.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { unregister(fns.ctx, id.raw()) })
}

fn mod_wheel(wire: &BbSuspensionWheel) -> SuspensionWheelQuery {
    SuspensionWheelQuery {
        loaded: wire.loaded != 0,
        compression_m: wire.compression_m,
        compression_speed_mps: wire.compression_speed_mps,
        steer_angle_rad: wire.steer_angle_rad,
        body_accel_up_mps2: wire.body_accel_up_mps2,
        tyre_lateral_force_n: wire.tyre_lateral_force_n,
        static_load_n: wire.static_load_n,
    }
}

fn mod_query(wire: &BbSuspensionQuery) -> SuspensionQuery {
    SuspensionQuery {
        axle: wire.axle_index,
        role: Role::from_raw(wire.role),
        vehicle_revision: wire.vehicle_revision,
        dt: wire.dt,
        rest_travel_m: wire.rest_travel_m,
        max_travel_m: wire.max_travel_m,
        corner_mass_kg: wire.corner_mass_kg,
        wheels: [mod_wheel(&wire.wheels[0]), mod_wheel(&wire.wheels[1])],
    }
}

fn wire_wheel(result: &SuspensionWheelResult) -> BbSuspensionWheelResult {
    BbSuspensionWheelResult {
        body_force_n: result.body_force_n,
        tyre_load_n: result.tyre_load_n,
        camber_rad: result.camber_rad,
        reserved: [0; 2],
    }
}

fn wire_result(result: &SuspensionResult) -> BbSuspensionResult {
    BbSuspensionResult {
        size: size_of::<BbSuspensionResult>() as u32,
        wheels: [wire_wheel(&result.wheels[0]), wire_wheel(&result.wheels[1])],
        reserved: [0; 8],
    }
}

/// The callback the host calls for each axle: runs the closure in `user`.
unsafe extern "C" fn suspension_trampoline<F>(
    user: *mut c_void,
    query: *const BbSuspensionQuery,
    result: *mut BbSuspensionResult,
) -> BbStatus
where
    F: FnMut(&mut Ctx, &SuspensionQuery) -> Option<SuspensionResult> + 'static,
{
    if user.is_null() || query.is_null() || result.is_null() {
        return BB_ERR_ARG;
    }
    // SAFETY: `user` is the `HookEntry<F>` that `register_suspension` kept alive (see there).
    let entry = unsafe { &*user.cast::<HookEntry<F>>() };
    // A call that arrives while the closure runs (a host call from inside it that steps the physics) is declined.
    let Ok(mut handler) = entry.handler.try_borrow_mut() else {
        return BB_SUSPENSION_NOT_HANDLED;
    };
    // SAFETY: the host passes a query valid for the bytes its size word declares.
    let Some(wire) = (unsafe { read_query::<BbSuspensionQuery>(query) }) else {
        return BB_ERR_ARG;
    };
    let question = mod_query(&wire);
    let mut ctx = Ctx::new(entry.host, entry.keep.clone());
    let outcome = guard::run(|| Ok((*handler)(&mut ctx, &question)));
    match finish(entry.host, "suspension hook", outcome) {
        Ok(Some(answer)) => {
            // SAFETY: the host passes a result valid for the bytes its size word declares.
            unsafe { write_result(result, &wire_result(&answer)) };
            BB_SUSPENSION_HANDLED
        }
        Ok(None) => BB_SUSPENSION_NOT_HANDLED,
        Err(error) => error.code(),
    }
}
