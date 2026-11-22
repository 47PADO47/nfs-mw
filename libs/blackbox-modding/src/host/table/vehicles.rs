//! `bb.vehicles` on the host side: the backend the game implements, and the trampolines behind `BbVehicles`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, slot_of, write_prefixed};
use crate::abi::{
    BB_ERR_ARG, BB_ERR_UNSUPPORTED, BB_SPEC_GEARS, BbIface, BbStatus, BbVec3, BbVehicleSpec, BbVehicleState,
    BbVehicles, IFACE_VEHICLES, IFACE_VEHICLES_VERSION,
};

/// The player's car as the game reports it. Units as in [`BbVehicleState`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VehicleState {
    /// -1 reverse, 0 neutral, 1 and up the forward gears.
    pub gear: i32,
    /// Position, metres, render space.
    pub position: [f32; 3],
    /// Linear velocity, metres per second, render space.
    pub velocity: [f32; 3],
    /// Heading, radians.
    pub heading: f32,
    /// Speed along the car, metres per second (negative in reverse).
    pub speed: f32,
    /// Engine speed, rpm.
    pub rpm: f32,
    /// Nitrous left, 0 to 1.
    pub nos: f32,
    /// Damage, 0 to 1.
    pub damage: f32,
    /// Body slip angle, radians, positive when the car moves to the left of its heading (1.1).
    pub slip_angle: f32,
    /// Steering, -1 to 1, positive to the left (1.1).
    pub steering: f32,
    /// The strongest hit of the latest physics step, 0 to 1 (1.1).
    pub impact: f32,
    /// At least one wheel touches the ground (1.1).
    pub on_ground: bool,
}

/// The player's car in physical numbers, as the game reports it (1.3). Units and meanings as in [`BbVehicleSpec`]; a number the
/// game does not have is 0.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VehicleSpecSheet {
    /// Changes whenever the car changes; starts at 1.
    pub revision: u32,
    pub mass_kg: f32,
    pub wheelbase_m: f32,
    /// Front then rear.
    pub track_m: [f32; 2],
    pub cg_height_m: f32,
    pub front_weight_fraction: f32,
    pub tyre_radius_m: [f32; 2],
    pub tyre_width_m: [f32; 2],
    pub peak_friction: [f32; 2],
    pub sliding_friction: [f32; 2],
    /// `BB_DRIVE_*`.
    pub drive_layout: u32,
    pub gear_count: u32,
    pub max_power_w: f32,
    pub max_power_rpm: f32,
    pub redline_rpm: f32,
    pub final_drive: f32,
    pub gear_ratios: [f32; BB_SPEC_GEARS],
    pub brake_torque_nm: [f32; 2],
    pub drag_area_m2: f32,
    pub downforce_area_m2: f32,
}

impl VehicleSpecSheet {
    fn to_wire(self) -> BbVehicleSpec {
        BbVehicleSpec {
            size: size_of::<BbVehicleSpec>() as u32,
            revision: self.revision,
            mass_kg: self.mass_kg,
            wheelbase_m: self.wheelbase_m,
            track_front_m: self.track_m[0],
            track_rear_m: self.track_m[1],
            cg_height_m: self.cg_height_m,
            front_weight_fraction: self.front_weight_fraction,
            tyre_radius_front_m: self.tyre_radius_m[0],
            tyre_radius_rear_m: self.tyre_radius_m[1],
            tyre_width_front_m: self.tyre_width_m[0],
            tyre_width_rear_m: self.tyre_width_m[1],
            peak_friction_front: self.peak_friction[0],
            peak_friction_rear: self.peak_friction[1],
            sliding_friction_front: self.sliding_friction[0],
            sliding_friction_rear: self.sliding_friction[1],
            drive_layout: self.drive_layout,
            gear_count: self.gear_count,
            max_power_w: self.max_power_w,
            max_power_rpm: self.max_power_rpm,
            redline_rpm: self.redline_rpm,
            final_drive: self.final_drive,
            gear_ratios: self.gear_ratios,
            brake_torque_front_nm: self.brake_torque_nm[0],
            brake_torque_rear_nm: self.brake_torque_nm[1],
            drag_area_m2: self.drag_area_m2,
            downforce_area_m2: self.downforce_area_m2,
            reserved: [0; 8],
        }
    }
}

/// What the game implements for `bb.vehicles`. `owner` is the id of the mod that made the call.
pub trait VehiclesBackend {
    /// The player's car.
    fn state(&self, owner: &str) -> BackendResult<VehicleState>;
    /// Move the player's car to the road nearest `position`, facing `heading`. Both already checked to be finite.
    fn teleport(&self, owner: &str, position: [f32; 3], heading: f32) -> BackendResult<()>;
    /// Clear the damage and the punctures of the player's car.
    fn repair(&self, owner: &str) -> BackendResult<()>;
    /// Spawn a traffic car near `position`. Returns its handle. The game answers `BB_ERR_UNSUPPORTED` until it can.
    fn spawn_traffic(&self, owner: &str, position: [f32; 3], heading: f32) -> BackendResult<u64>;
    /// Add `amount` of a full tank (already checked to be finite and 0 to 1) to the player's nitrous, up to a full tank. A car
    /// without a nitrous system ignores it. Queued like a repair (1.1).
    fn add_nos(&self, owner: &str, amount: f32) -> BackendResult<()>;
    /// Turn the drift assist of the player's car on or off (1.2). Queued like a repair. A game without the assist keeps the default,
    /// which answers `BB_ERR_UNSUPPORTED`.
    fn set_drift_assist(&self, _owner: &str, _on: bool) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The physical summary of the player's car (1.3). `NotReady` while no car is driven. A game without it keeps the default,
    /// which answers `BB_ERR_UNSUPPORTED`.
    fn spec(&self, _owner: &str) -> BackendResult<VehicleSpecSheet> {
        Err(BB_ERR_UNSUPPORTED)
    }
}

/// Build the `bb.vehicles` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn VehiclesBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbVehicles {
        head: BbIface {
            size: size_of::<BbVehicles>() as u32,
            major: IFACE_VEHICLES_VERSION.0,
            minor: IFACE_VEHICLES_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        state: Some(state),
        teleport: Some(teleport),
        repair: Some(repair),
        spawn_traffic: Some(spawn_traffic),
        add_nos: Some(add_nos),
        set_drift_assist: Some(set_drift_assist),
        spec: Some(spec),
    });
    // Nothing the mod registers stays in the game, so unloading removes nothing; the release keeps the slot alive.
    let kept = slot.clone();
    let release = Box::new(move || kept.release_with(|_, _, _| {}));
    Attached::new(IFACE_VEHICLES, IFACE_VEHICLES_VERSION, table, release)
}

/// A point the mod sends: three finite numbers.
fn point(value: BbVec3) -> Result<[f32; 3], BbStatus> {
    let point = [value.x, value.y, value.z];
    if !point.iter().all(|coord| coord.is_finite()) {
        return Err(BB_ERR_ARG);
    }
    Ok(point)
}

/// A value the mod sends that must be finite.
fn finite(value: f32) -> Result<f32, BbStatus> {
    if !value.is_finite() {
        return Err(BB_ERR_ARG);
    }
    Ok(value)
}

/// `state`: write the player's car to `out`, as far as the mod's size word allows.
unsafe extern "C" fn state(ctx: *mut c_void, out: *mut BbVehicleState) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        let car = slot.backend.state(&slot.owner)?;
        let wire = BbVehicleState {
            size: size_of::<BbVehicleState>() as u32,
            gear: car.gear,
            position: BbVec3 { x: car.position[0], y: car.position[1], z: car.position[2] },
            velocity: BbVec3 { x: car.velocity[0], y: car.velocity[1], z: car.velocity[2] },
            heading: car.heading,
            speed: car.speed,
            rpm: car.rpm,
            nos: car.nos,
            damage: car.damage,
            slip_angle: car.slip_angle,
            steering: car.steering,
            impact: car.impact,
            on_ground: u32::from(car.on_ground),
        };
        // SAFETY: `out` is the mod's struct; `write_prefixed` checks its size word before it writes.
        unsafe { write_prefixed(out, &wire) }
    })
}

/// `spec`: write the physical summary of the player's car to `out`, as far as the mod's size word allows.
unsafe extern "C" fn spec(ctx: *mut c_void, out: *mut BbVehicleSpec) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        let sheet = slot.backend.spec(&slot.owner)?;
        // SAFETY: `out` is the mod's struct; `write_prefixed` checks its size word before it writes.
        unsafe { write_prefixed(out, &sheet.to_wire()) }
    })
}

/// `teleport`: queue the player's car to move.
unsafe extern "C" fn teleport(ctx: *mut c_void, position: BbVec3, heading: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        let position = point(position)?;
        slot.backend.teleport(&slot.owner, position, finite(heading)?)
    })
}

/// `repair`: clear the player's car's damage.
unsafe extern "C" fn repair(ctx: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        slot.backend.repair(&slot.owner)
    })
}

/// `add_nos`: queue nitrous for the player's car. The amount is a part of a full tank, finite and 0 to 1.
unsafe extern "C" fn add_nos(ctx: *mut c_void, amount: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        if !(0.0..=1.0).contains(&finite(amount)?) {
            return Err(BB_ERR_ARG);
        }
        slot.backend.add_nos(&slot.owner, amount)
    })
}

/// `set_drift_assist`: queue the assist of the player's car on (any non-zero value) or off.
unsafe extern "C" fn set_drift_assist(ctx: *mut c_void, enabled: u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        slot.backend.set_drift_assist(&slot.owner, enabled != 0)
    })
}

/// `spawn_traffic`: a new traffic car; its handle goes to `out_id`.
unsafe extern "C" fn spawn_traffic(ctx: *mut c_void, position: BbVec3, heading: f32, out_id: *mut u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        let handle = slot.backend.spawn_traffic(&slot.owner, point(position)?, finite(heading)?)?;
        // SAFETY: `out_id` is non-null (checked above) and valid for one write (the mod's contract).
        unsafe { out_id.write(handle) };
        Ok(())
    })
}
