//! `bb.handling` on the mod side: read the table, and register a Rust closure as the tyre hook. The suspension and controls hooks
//! are in `handling_suspension.rs` and `handling_controls.rs`; they share the helpers here.

use core::cell::RefCell;
use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::null_mut;

use super::{HostTable, finish, read_or, status_result};
use crate::abi::{
    BB_ERR_ARG, BB_TYRE_HANDLED, BB_TYRE_NOT_HANDLED, BbHandling, BbHandlingRegisterControlsFn,
    BbHandlingRegisterSuspensionFn, BbHandlingRegisterTyreFn, BbHandlingUnregisterControlsFn,
    BbHandlingUnregisterSuspensionFn, BbHandlingUnregisterTyreFn, BbStatus, BbTyreHook, BbTyreQuery, BbTyreResult,
};
use crate::sdk::ctx::Keep;
use crate::sdk::{Ctx, Error, HandlingHookId, Interface, ModResult, Role, TyreQuery, TyreResult, guard};

/// The function pointers of one `bb.handling` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct HandlingFns {
    pub(super) ctx: *mut c_void,
    pub(super) register_tyre: Option<BbHandlingRegisterTyreFn>,
    pub(super) unregister_tyre: Option<BbHandlingUnregisterTyreFn>,
    pub(super) register_suspension: Option<BbHandlingRegisterSuspensionFn>,
    pub(super) unregister_suspension: Option<BbHandlingUnregisterSuspensionFn>,
    pub(super) register_controls: Option<BbHandlingRegisterControlsFn>,
    pub(super) unregister_controls: Option<BbHandlingUnregisterControlsFn>,
}

/// Read the `bb.handling` table the host offered.
pub(crate) fn read(iface: Interface) -> HandlingFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        HandlingFns {
            ctx: read_or(&iface, offset_of!(BbHandling, ctx), null_mut()),
            register_tyre: read_or(&iface, offset_of!(BbHandling, register_tyre), None),
            unregister_tyre: read_or(&iface, offset_of!(BbHandling, unregister_tyre), None),
            register_suspension: read_or(&iface, offset_of!(BbHandling, register_suspension), None),
            unregister_suspension: read_or(&iface, offset_of!(BbHandling, unregister_suspension), None),
            register_controls: read_or(&iface, offset_of!(BbHandling, register_controls), None),
            unregister_controls: read_or(&iface, offset_of!(BbHandling, unregister_controls), None),
        }
    }
}

/// A registered hook: the closure, and what its trampoline needs to call it.
pub(super) struct HookEntry<F> {
    pub(super) host: HostTable,
    pub(super) keep: Keep,
    pub(super) handler: RefCell<F>,
}

/// Register a closure as the tyre hook. The closure is kept in `keep` until the mod unloads.
pub(crate) fn register_tyre<F>(fns: &HandlingFns, host: HostTable, keep: &Keep, handler: F) -> ModResult<HandlingHookId>
where
    F: FnMut(&mut Ctx, &TyreQuery) -> Option<TyreResult> + 'static,
{
    let register = fns.register_tyre.ok_or(Error::Unsupported)?;
    let entry = Box::new(HookEntry { host, keep: keep.clone(), handler: RefCell::new(handler) });
    let user: *mut c_void = (&*entry as *const HookEntry<F>).cast_mut().cast();
    let hook = BbTyreHook {
        size: size_of::<BbTyreHook>() as u32,
        flags: 0,
        user,
        callback: Some(tyre_trampoline::<F>),
        reserved: [0; 4],
    };
    let mut id = 0_u64;
    // SAFETY: the host calls `tyre_trampoline::<F>` with `user` only while the hook is registered. `entry` moves into `keep`
    // below, which lives until the mod's `destroy`; the host retires the hook before the mod unloads. `hook` and `id` are
    // valid for the call.
    let status = unsafe { register(fns.ctx, &hook, &mut id) };
    status_result(status)?;
    keep.borrow_mut().push(entry);
    Ok(HandlingHookId::from_raw(id))
}

/// Remove the tyre hook this mod registered.
pub(crate) fn unregister_tyre(fns: &HandlingFns, id: HandlingHookId) -> ModResult<()> {
    let unregister = fns.unregister_tyre.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { unregister(fns.ctx, id.raw()) })
}

/// Copy the query the host wrote over a zeroed one, as far as the host's size word goes, so a field a newer minor appended
/// to the layout is zero for a host that predates it.
///
/// # Safety
/// `query` must be non-null and valid for reads of the bytes its size word declares.
pub(super) unsafe fn read_query<T: Copy + Default>(query: *const T) -> Option<T> {
    // SAFETY: the size word is the first field and the host wrote it.
    let declared = unsafe { query.cast::<u32>().read_unaligned() } as usize;
    if declared < size_of::<u32>() {
        return None;
    }
    let mut out = T::default();
    let len = declared.min(size_of::<T>());
    // SAFETY: `query` is valid for `declared` bytes and `out` for its whole size; `len` is at most both.
    unsafe { core::ptr::copy_nonoverlapping(query.cast::<u8>(), core::ptr::from_mut(&mut out).cast::<u8>(), len) };
    Some(out)
}

/// Write the fields of `answer` after the size word, as far as the host's size word allows.
///
/// # Safety
/// `result` must be non-null and valid for writes of the bytes its size word declares.
pub(super) unsafe fn write_result<T: Copy>(result: *mut T, answer: &T) {
    // SAFETY: the size word is the first field and the host wrote it.
    let allowed = unsafe { result.cast::<u32>().read_unaligned() } as usize;
    let word = size_of::<u32>();
    let len = allowed.min(size_of::<T>());
    if len <= word {
        return;
    }
    // SAFETY: `result` is valid for `allowed` bytes, `answer` for its whole size, and `len` is at most both.
    unsafe {
        core::ptr::copy_nonoverlapping(
            core::ptr::from_ref(answer).cast::<u8>().add(word),
            result.cast::<u8>().add(word),
            len - word,
        );
    }
}

fn mod_query(wire: &BbTyreQuery) -> TyreQuery {
    TyreQuery {
        wheel_index: wire.wheel_index,
        role: Role::from_raw(wire.role),
        loaded: wire.loaded != 0,
        vehicle_revision: wire.vehicle_revision,
        surface_tag: wire.surface_tag,
        dt: wire.dt,
        vertical_load_n: wire.vertical_load_n,
        patch_lateral_velocity: wire.patch_lateral_velocity,
        patch_forward_velocity: wire.patch_forward_velocity,
        body_speed: wire.body_speed,
        wheel_spin: wire.wheel_spin,
        drive_torque_nm: wire.drive_torque_nm,
        brake_torque_nm: wire.brake_torque_nm,
        wheel_radius_m: wire.wheel_radius_m,
        wheel_inertia_kgm2: wire.wheel_inertia_kgm2,
        corner_mass_kg: wire.corner_mass_kg,
        surface_lateral: wire.surface_lateral,
        surface_drive: wire.surface_drive,
        surface_rolling: wire.surface_rolling,
        tyre_condition: wire.tyre_condition,
        ambient_temperature_c: wire.ambient_temperature_c,
        road_temperature_c: wire.road_temperature_c,
        camber_rad: wire.camber_rad,
    }
}

fn wire_result(result: &TyreResult) -> BbTyreResult {
    BbTyreResult {
        size: size_of::<BbTyreResult>() as u32,
        longitudinal_force_n: result.longitudinal_force_n,
        lateral_force_n: result.lateral_force_n,
        aligning_torque_nm: result.aligning_torque_nm,
        wheel_spin: result.wheel_spin,
        traction: result.traction,
        slip: result.slip,
        slip_angle: result.slip_angle,
        reserved: [0; 8],
    }
}

/// The callback the host calls for each wheel: runs the closure in `user`.
unsafe extern "C" fn tyre_trampoline<F>(
    user: *mut c_void,
    query: *const BbTyreQuery,
    result: *mut BbTyreResult,
) -> BbStatus
where
    F: FnMut(&mut Ctx, &TyreQuery) -> Option<TyreResult> + 'static,
{
    if user.is_null() || query.is_null() || result.is_null() {
        return BB_ERR_ARG;
    }
    // SAFETY: `user` is the `HookEntry<F>` that `register_tyre` kept alive (see there).
    let entry = unsafe { &*user.cast::<HookEntry<F>>() };
    // A call that arrives while the closure runs (a host call from inside it that steps the physics) is declined.
    let Ok(mut handler) = entry.handler.try_borrow_mut() else {
        return BB_TYRE_NOT_HANDLED;
    };
    // SAFETY: the host passes a query valid for the bytes its size word declares.
    let Some(wire) = (unsafe { read_query::<BbTyreQuery>(query) }) else {
        return BB_ERR_ARG;
    };
    let question = mod_query(&wire);
    let mut ctx = Ctx::new(entry.host, entry.keep.clone());
    let outcome = guard::run(|| Ok((*handler)(&mut ctx, &question)));
    match finish(entry.host, "tyre hook", outcome) {
        Ok(Some(answer)) => {
            // SAFETY: the host passes a result valid for the bytes its size word declares.
            unsafe { write_result(result, &wire_result(&answer)) };
            BB_TYRE_HANDLED
        }
        Ok(None) => BB_TYRE_NOT_HANDLED,
        Err(error) => error.code(),
    }
}
