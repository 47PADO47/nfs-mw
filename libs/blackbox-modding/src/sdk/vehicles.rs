//! `Vehicles`: the player's car, read and moved (catalogue: docs/modding/api-world.md#vehicles-and-the-player).

use super::ffi::vehicles::{self as host_vehicles, VehiclesFns};
use super::{ModResult, VehicleId};

/// The player's car at one moment, in render space (x forward, y left, z up; metres and seconds).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VehicleState {
    /// -1 reverse, 0 neutral, 1 and up the forward gears.
    pub gear: i32,
    /// Position, metres.
    pub position: [f32; 3],
    /// Linear velocity, metres per second.
    pub velocity: [f32; 3],
    /// Heading in radians, from +x towards +y.
    pub heading: f32,
    /// Speed along the car, metres per second (negative in reverse).
    pub speed: f32,
    /// Engine speed, revolutions per minute.
    pub rpm: f32,
    /// Nitrous left, 0 to 1.
    pub nos: f32,
    /// Damage, 0 undamaged to 1.
    pub damage: f32,
    /// Body slip angle in radians: the angle between where the car points and where it moves, positive when it moves to the
    /// left of its heading; 0 below 1 m/s forward speed. Needs `bb.vehicles` 1.1.
    pub slip_angle: f32,
    /// Steering, -1 to 1, positive to the left. Needs 1.1.
    pub steering: f32,
    /// The strongest hit of the latest physics step against the world or an object, 0 to 1. Needs 1.1.
    pub impact: f32,
    /// At least one wheel touches the ground. Needs 1.1.
    pub on_ground: bool,
}

/// Which axles of a car are driven.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DriveLayout {
    /// The front axle.
    Front,
    /// The rear axle.
    #[default]
    Rear,
    /// Both axles.
    All,
}

/// The player's car in physical numbers, in SI units (`bb.vehicles` 1.3). Every value is the host's best estimate from the
/// car's data and a mod may ignore any of them. A number the host does not have is 0 (the catalogue lists which). Axle pairs
/// are front then rear.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VehicleSpec {
    /// Changes whenever the car changes; the tyre query carries the same number as `vehicle_revision`.
    pub revision: u32,
    /// Mass of the whole car (kg).
    pub mass_kg: f32,
    /// Distance between the axles (m).
    pub wheelbase_m: f32,
    /// Distance between the tyre centres of the front and the rear axle (m).
    pub track_m: [f32; 2],
    /// Height of the centre of gravity above the road at rest (m).
    pub cg_height_m: f32,
    /// Share of the weight on the front axle at rest, 0 to 1.
    pub front_weight_fraction: f32,
    /// Rolling radius of the front and the rear tyre (m).
    pub tyre_radius_m: [f32; 2],
    /// Section width of the front and the rear tyre (m).
    pub tyre_width_m: [f32; 2],
    /// Friction coefficient of the front and the rear tyre at the limit of grip (dimensionless).
    pub peak_friction: [f32; 2],
    /// Friction coefficient of the front and the rear tyre while sliding (dimensionless).
    pub sliding_friction: [f32; 2],
    /// Which axles are driven.
    pub drive: DriveLayout,
    /// The highest engine power (W), before forced induction and nitrous.
    pub max_power_w: f32,
    /// The engine speed of that power (rpm).
    pub max_power_rpm: f32,
    /// The rev limiter (rpm).
    pub redline_rpm: f32,
    /// The final drive ratio.
    pub final_drive: f32,
    /// The forward gear ratios from first gear, without the final drive (at most 8).
    pub gear_ratios: Vec<f32>,
    /// Brake torque at one wheel of the front and the rear axle (N m).
    pub brake_torque_nm: [f32; 2],
    /// Drag area `Cd * A` (m^2); 0 when unknown.
    pub drag_area_m2: f32,
    /// Lift area of the downforce (m^2); 0 when unknown.
    pub downforce_area_m2: f32,
}

/// The `bb.vehicles` interface of the host, as a mod uses it. Obtained from `Ctx::vehicles`.
#[derive(Clone, Copy)]
pub struct Vehicles {
    fns: VehiclesFns,
}

impl Vehicles {
    pub(crate) fn new(fns: VehiclesFns) -> Self {
        Vehicles { fns }
    }

    /// The player's car now.
    pub fn player(&self) -> ModResult<VehicleState> {
        host_vehicles::state(&self.fns)
    }

    /// Move the player's car to the road nearest `position`, facing `heading`. The move is made after the mods run.
    pub fn teleport_player(&self, position: [f32; 3], heading: f32) -> ModResult<()> {
        host_vehicles::teleport(&self.fns, position, heading)
    }

    /// Clear the player's car's damage and punctures.
    pub fn repair_player(&self) -> ModResult<()> {
        host_vehicles::repair(&self.fns)
    }

    /// Add `amount` (a part of a full tank, 0 to 1) to the player's nitrous, up to a full tank. The change is made after the
    /// mods run; a car without a nitrous system ignores it. Needs `bb.vehicles` 1.1.
    pub fn add_nos(&self, amount: f32) -> ModResult<()> {
        host_vehicles::add_nos(&self.fns, amount)
    }

    /// Turn the drift assist of the player's car on or off: a physics layer that makes a slide easy to start with the handbrake
    /// and to hold with the wheel. It is off until a mod asks, applies to whatever car the player drives, and stays as set. The
    /// change is made after the mods run. Needs `bb.vehicles` 1.2 (`Unsupported` on an older host).
    pub fn set_drift_assist(&self, on: bool) -> ModResult<()> {
        host_vehicles::set_drift_assist(&self.fns, on)
    }

    /// The player's car in physical numbers: mass, geometry, tyres, engine, gearing, brakes. Read it when the car changes (its
    /// `revision` changes) and build the model's parameters once. `NotReady` while no car is driven. Needs `bb.vehicles` 1.3
    /// (`Unsupported` on an older host).
    pub fn spec(&self) -> ModResult<VehicleSpec> {
        host_vehicles::spec(&self.fns).map(host_vehicles::vehicle_spec)
    }

    /// Spawn a traffic car near `position`. The host answers `Unsupported` in 1.0 (see the catalogue).
    pub fn spawn_traffic(&self, position: [f32; 3], heading: f32) -> ModResult<VehicleId> {
        host_vehicles::spawn_traffic(&self.fns, position, heading).map(VehicleId::from_raw)
    }
}
