//! `bb.vehicles` on the mod side: read the table, read the player's car, move it, repair it.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{
    BB_DRIVE_ALL, BB_DRIVE_FRONT, BbVec3, BbVehicleAddNosFn, BbVehicleRepairFn, BbVehicleSetDriftAssistFn,
    BbVehicleSpawnTrafficFn, BbVehicleSpec, BbVehicleSpecFn, BbVehicleState, BbVehicleStateFn, BbVehicleTeleportFn,
    BbVehicles,
};
use crate::sdk::{DriveLayout, Error, Interface, ModResult, VehicleSpec, VehicleState};

/// The function pointers of one `bb.vehicles` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct VehiclesFns {
    ctx: *mut c_void,
    state: Option<BbVehicleStateFn>,
    teleport: Option<BbVehicleTeleportFn>,
    repair: Option<BbVehicleRepairFn>,
    spawn_traffic: Option<BbVehicleSpawnTrafficFn>,
    add_nos: Option<BbVehicleAddNosFn>,
    set_drift_assist: Option<BbVehicleSetDriftAssistFn>,
    spec: Option<BbVehicleSpecFn>,
}

/// Read the `bb.vehicles` table the host offered.
pub(crate) fn read(iface: Interface) -> VehiclesFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        VehiclesFns {
            ctx: read_or(&iface, offset_of!(BbVehicles, ctx), null_mut()),
            state: read_or(&iface, offset_of!(BbVehicles, state), None),
            teleport: read_or(&iface, offset_of!(BbVehicles, teleport), None),
            repair: read_or(&iface, offset_of!(BbVehicles, repair), None),
            spawn_traffic: read_or(&iface, offset_of!(BbVehicles, spawn_traffic), None),
            add_nos: read_or(&iface, offset_of!(BbVehicles, add_nos), None),
            set_drift_assist: read_or(&iface, offset_of!(BbVehicles, set_drift_assist), None),
            spec: read_or(&iface, offset_of!(BbVehicles, spec), None),
        }
    }
}

/// The player's car. The mod sets the size word to its own layout; the host writes what it has of it.
pub(crate) fn state(fns: &VehiclesFns) -> ModResult<VehicleState> {
    let state = fns.state.ok_or(Error::Unsupported)?;
    let mut raw = BbVehicleState { size: size_of::<BbVehicleState>() as u32, ..BbVehicleState::default() };
    // SAFETY: `raw` is valid for one write of its whole layout, and its size word says so.
    status_result(unsafe { state(fns.ctx, &mut raw) })?;
    Ok(VehicleState {
        gear: raw.gear,
        position: raw.position.to_array(),
        velocity: raw.velocity.to_array(),
        heading: raw.heading,
        speed: raw.speed,
        rpm: raw.rpm,
        nos: raw.nos,
        damage: raw.damage,
        slip_angle: raw.slip_angle,
        steering: raw.steering,
        impact: raw.impact,
        on_ground: raw.on_ground != 0,
    })
}

/// The player's car in physical numbers. The mod sets the size word to its own layout; the host writes what it has of it.
pub(crate) fn spec(fns: &VehiclesFns) -> ModResult<BbVehicleSpec> {
    let spec = fns.spec.ok_or(Error::Unsupported)?;
    let mut raw = BbVehicleSpec { size: size_of::<BbVehicleSpec>() as u32, ..BbVehicleSpec::default() };
    // SAFETY: `raw` is valid for one write of its whole layout, and its size word says so.
    status_result(unsafe { spec(fns.ctx, &mut raw) })?;
    Ok(raw)
}

/// The safe form of the wire spec.
pub(crate) fn vehicle_spec(raw: BbVehicleSpec) -> VehicleSpec {
    let gears = (raw.gear_count as usize).min(raw.gear_ratios.len());
    let drive = match raw.drive_layout {
        BB_DRIVE_FRONT => DriveLayout::Front,
        BB_DRIVE_ALL => DriveLayout::All,
        _ => DriveLayout::Rear,
    };
    VehicleSpec {
        revision: raw.revision,
        mass_kg: raw.mass_kg,
        wheelbase_m: raw.wheelbase_m,
        track_m: [raw.track_front_m, raw.track_rear_m],
        cg_height_m: raw.cg_height_m,
        front_weight_fraction: raw.front_weight_fraction,
        tyre_radius_m: [raw.tyre_radius_front_m, raw.tyre_radius_rear_m],
        tyre_width_m: [raw.tyre_width_front_m, raw.tyre_width_rear_m],
        peak_friction: [raw.peak_friction_front, raw.peak_friction_rear],
        sliding_friction: [raw.sliding_friction_front, raw.sliding_friction_rear],
        drive,
        max_power_w: raw.max_power_w,
        max_power_rpm: raw.max_power_rpm,
        redline_rpm: raw.redline_rpm,
        final_drive: raw.final_drive,
        gear_ratios: raw.gear_ratios[..gears].to_vec(),
        brake_torque_nm: [raw.brake_torque_front_nm, raw.brake_torque_rear_nm],
        drag_area_m2: raw.drag_area_m2,
        downforce_area_m2: raw.downforce_area_m2,
    }
}

/// Queue a move of the player's car.
pub(crate) fn teleport(fns: &VehiclesFns, position: [f32; 3], heading: f32) -> ModResult<()> {
    let teleport = fns.teleport.ok_or(Error::Unsupported)?;
    let at = BbVec3 { x: position[0], y: position[1], z: position[2] };
    // SAFETY: the table is valid while the mod is loaded; the arguments are plain values.
    status_result(unsafe { teleport(fns.ctx, at, heading) })
}

/// Repair the player's car.
pub(crate) fn repair(fns: &VehiclesFns) -> ModResult<()> {
    let repair = fns.repair.ok_or(Error::Unsupported)?;
    // SAFETY: as for `teleport`.
    status_result(unsafe { repair(fns.ctx) })
}

/// Queue nitrous for the player's car.
pub(crate) fn add_nos(fns: &VehiclesFns, amount: f32) -> ModResult<()> {
    let add = fns.add_nos.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded; the argument is a plain value.
    status_result(unsafe { add(fns.ctx, amount) })
}

/// Turn the drift assist of the player's car on or off.
pub(crate) fn set_drift_assist(fns: &VehiclesFns, on: bool) -> ModResult<()> {
    let set = fns.set_drift_assist.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded; the argument is a plain value.
    status_result(unsafe { set(fns.ctx, u32::from(on)) })
}

/// Spawn a traffic car near `position`; returns the raw handle.
pub(crate) fn spawn_traffic(fns: &VehiclesFns, position: [f32; 3], heading: f32) -> ModResult<u64> {
    let spawn = fns.spawn_traffic.ok_or(Error::Unsupported)?;
    let at = BbVec3 { x: position[0], y: position[1], z: position[2] };
    let mut id = 0_u64;
    // SAFETY: `id` is valid for one write; the other arguments are plain values.
    status_result(unsafe { spawn(fns.ctx, at, heading, &mut id) })?;
    Ok(id)
}
