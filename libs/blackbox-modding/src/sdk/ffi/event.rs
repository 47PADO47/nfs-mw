//! Decode a `BbEvent` into the typed `Event` (ADR 0005 section 4). Only the bytes the event's sizes cover are read. A
//! payload the host did not fill in far enough decodes as `Event::Unknown`, so a mod never reads past what it was given.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::addr_of;

use crate::abi::*;
use crate::sdk::{AbilityId, ActionId, Collider, Event, MenuId, Role, VehicleId, WorldId};

/// Decode one event. `None` when the event header itself is too short or null; a kind or payload the SDK does not
/// describe is `Some(Event::Unknown)`.
///
/// # Safety
/// `raw` must be null or valid for reads of the bytes its `size` field declares. A payload pointer, when the size covers
/// it, must be valid for reads of the bytes its own size word declares.
pub(crate) unsafe fn decode<'a>(raw: *const BbEvent) -> Option<Event<'a>> {
    if raw.is_null() {
        return None;
    }
    // SAFETY: the size word is the first field of every version of the event (see `# Safety`).
    let size = unsafe { addr_of!((*raw).size).read() };
    if !has_field(size, offset_of!(BbEvent, frame) + size_of::<u64>()) {
        return None;
    }
    // SAFETY: the size covers `kind` and `frame` (checked above).
    let (kind, frame) = unsafe { (addr_of!((*raw).kind).read(), addr_of!((*raw).frame).read()) };
    let mut payload: *const c_void = core::ptr::null();
    if has_field(size, offset_of!(BbEvent, payload) + size_of::<*const c_void>()) {
        // SAFETY: the size covers `payload` (checked above).
        payload = unsafe { addr_of!((*raw).payload).read() };
    }
    // SAFETY: the payload is null or valid for its own size (see `# Safety`).
    let typed = unsafe { decode_payload(kind, frame, payload) };
    Some(typed.unwrap_or(Event::Unknown { kind, frame }))
}

/// Read the payload of `kind` and build the typed event. `None` when the kind is unknown or the payload is missing,
/// short or not valid.
///
/// # Safety
/// `payload` must be null, or valid for reads of the bytes its own size word declares.
unsafe fn decode_payload<'a>(kind: u32, frame: u64, payload: *const c_void) -> Option<Event<'a>> {
    match kind {
        BB_EVENT_FRAME_START => {
            // SAFETY: forwarded to the caller's guarantee.
            let p = unsafe { payload_of::<BbFrameStart>(payload) }?;
            Some(Event::FrameStart { frame, dt_real: p.dt_real })
        }
        BB_EVENT_SCENE_TICK => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbSceneTick>(payload) }?;
            Some(Event::SceneTick { frame, dt_sim: p.dt_sim, time_scale: p.time_scale, paused: p.paused != 0 })
        }
        BB_EVENT_WORLD_LOADED => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbWorldLoaded>(payload) }?;
            Some(Event::WorldLoaded {
                frame,
                world: WorldId::from_raw(p.world),
                streaming_ready: p.streaming_ready != 0,
            })
        }
        BB_EVENT_WORLD_UNLOADING => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbWorldUnloading>(payload) }?;
            Some(Event::WorldUnloading { frame, world: WorldId::from_raw(p.world) })
        }
        BB_EVENT_VEHICLE_SPAWNED => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbVehicleSpawned>(payload) }?;
            Some(Event::VehicleSpawned {
                frame,
                vehicle: VehicleId::from_raw(p.vehicle),
                role: Role::from_raw(p.role)?,
                car_id: p.car_id,
            })
        }
        BB_EVENT_VEHICLE_DESPAWNED => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbVehicleDespawned>(payload) }?;
            Some(Event::VehicleDespawned {
                frame,
                vehicle: VehicleId::from_raw(p.vehicle),
                role: Role::from_raw(p.role)?,
            })
        }
        BB_EVENT_VEHICLE_COLLISION => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbVehicleCollision>(payload) }?;
            Some(Event::VehicleCollision {
                frame,
                vehicle: VehicleId::from_raw(p.vehicle),
                other: Collider::from_raw(p.other_kind)?,
                other_id: p.other_id,
                impulse: p.impulse,
                point: p.point,
            })
        }
        BB_EVENT_HOUR_CHANGED => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbHourChanged>(payload) }?;
            Some(Event::HourChanged { frame, hour: p.hour })
        }
        BB_EVENT_MENU_OPENED => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbMenuEvent>(payload) }?;
            Some(Event::MenuOpened { frame, menu: MenuId::from_raw(p.menu) })
        }
        BB_EVENT_MENU_CLOSED => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbMenuEvent>(payload) }?;
            Some(Event::MenuClosed { frame, menu: MenuId::from_raw(p.menu) })
        }
        BB_EVENT_ACTION_PRESSED => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbActionPressed>(payload) }?;
            // SAFETY: the host's name is valid for this call (ADR 0005 section 4).
            let name = unsafe { p.name.as_str() }?;
            Some(Event::ActionPressed { frame, action: ActionId::from_raw(p.action), name })
        }
        BB_EVENT_ABILITY_ACTIVATED => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbAbilityActivated>(payload) }?;
            Some(Event::AbilityActivated { frame, ability: AbilityId::from_raw(p.ability) })
        }
        BB_EVENT_MOD_UNLOADING => {
            // SAFETY: as above.
            let p = unsafe { payload_of::<BbModUnloading>(payload) }?;
            // SAFETY: the host's id is valid for this call (ADR 0005 section 4).
            let mod_id = unsafe { p.mod_id.as_str() }?;
            Some(Event::ModUnloading { frame, mod_id })
        }
        _ => None,
    }
}

/// Copy the payload struct `T` out of `payload`, when the payload's own size word covers all of `T`.
///
/// # Safety
/// `payload` must be null, or valid for reads of the bytes its size word declares.
unsafe fn payload_of<T: Copy>(payload: *const c_void) -> Option<T> {
    if payload.is_null() {
        return None;
    }
    // SAFETY: every payload starts with its size word (see `# Safety`).
    let size = unsafe { payload.cast::<u32>().read_unaligned() };
    if (size as usize) < size_of::<T>() {
        return None;
    }
    // SAFETY: the size word covers `T` (checked above).
    Some(unsafe { payload.cast::<T>().read_unaligned() })
}

#[cfg(test)]
mod tests;
