//! What a car's bundle says about drifting: when its physics calls a slide a drift, and what a drift pays into the
//! nitrous tank. Spec: `docs/formats/chameleon-drift.md`.
//!
//! Two places hold the numbers, both Genesys objects of the car's own bundle (see [`crate::genesys`]):
//!
//! - the handling definition has a `DriftBehaviour` whose `Drift_trigger` and `Drift_exit` structures say when a slide
//!   starts and stops being a drift (speed, slip angle, a hold time). They differ a little from car to car;
//! - the `Vehicle_Gameplay` object names the nitrous parameter objects of the car, and each has a `Drifting` structure
//!   with the payout. The same objects are in every car of the install.
//!
//! Field names are hashed, so fields are read at fixed offsets and their meaning is inferred; the doc tags each one.

use chameleon_bundle::{Bundle, Resource, ResourceKind};

use crate::error::{Error, Result};
use crate::genesys::{Object, TypeNames, find_object};

/// The top bit of an import slot offset: the slot names a resource (a type or an object), not a pointer.
const HANDLE_FLAG: u32 = 0x8000_0000;
/// The slots of `Vehicle_Gameplay` that hold the car's nitrous parameter objects: the stock nitrous, then the burn nitrous.
const STOCK_SLOT: u32 = 0x74;
const BURN_SLOT: u32 = 0x7C;
/// A full nitrous tank in the units the parameters use (a fatal hit refills `100`, the restrictions name `100`).
pub const TANK_UNITS: f32 = 100.0;
/// Metres per second in one mile per hour.
const MPS_PER_MPH: f32 = 0.447_04;

/// When the physics of one car calls a slide a drift. Speeds are metres per second, angles degrees, times seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftPhysics {
    /// The speed under which a slide does not start a drift (`Drift_trigger +0x10`; 10 in 92 of 93 cars).
    pub trigger_speed: f32,
    /// The slip angle from which a slide starts a drift (`Drift_trigger +0x18`; 5.5 to 12, median 7.5).
    pub trigger_angle: f32,
    /// The slip angle under which a drift ends (`Drift_exit +0x10`; 2 to 5, 3 in 71 cars).
    pub exit_angle: f32,
    /// How long the angle has to stay under the exit angle (`Drift_exit +0x24`; 0.25 in 81 cars, 0.1 in 10, 0 in 2).
    pub exit_hold: f32,
}

impl DriftPhysics {
    /// The median of the 93 cars of the install: what a car without a bundle to read is given.
    pub const TYPICAL: DriftPhysics =
        DriftPhysics { trigger_speed: 10.0, trigger_angle: 7.5, exit_angle: 3.0, exit_hold: 0.25 };
}

/// What drifting pays one nitrous type, from its `NitrousParameters.Drifting` structure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftPayout {
    /// The id of the HUD message that shows the gain (`+0x0C`; 24010, "DRIFTING {distance}" in the text tables).
    pub message: u32,
    /// A factor on the gain (`+0x10`; 1 in every object).
    pub scale: f32,
    /// The speed under which a drift pays nothing, in miles per hour (`+0x14`; 30) **[unconfirmed: the unit]**.
    pub min_speed_mph: f32,
    /// Units of a tank of [`TANK_UNITS`] gained a second (`+0x18`; 5 stock, 6 burn) **[unconfirmed: a second]**.
    pub gain: f32,
    /// Seconds a drift has to last before it pays (`+0x1C`; 0.75) **[unconfirmed: the meaning]**.
    pub delay: f32,
    /// The flag at `+0x20`: 0 in the stock nitrous, 1 in the burn nitrous.
    pub flag: bool,
}

impl DriftPayout {
    /// The payout of the burn nitrous, as every car of the install has it.
    pub const BURN: DriftPayout =
        DriftPayout { message: 24010, scale: 1.0, min_speed_mph: 30.0, gain: 6.0, delay: 0.75, flag: true };

    /// Fractions of a full tank gained a second.
    pub fn tank_per_second(&self) -> f32 {
        self.scale * self.gain / TANK_UNITS
    }

    /// The speed under which a drift pays nothing, in metres per second.
    pub fn min_speed(&self) -> f32 {
        self.min_speed_mph * MPS_PER_MPH
    }
}

/// The drift payout of a car's two nitrous parameter objects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftNitrous {
    /// The stock nitrous (`Vehicle_Gameplay +0x74`).
    pub stock: DriftPayout,
    /// The burn nitrous (`Vehicle_Gameplay +0x7C`).
    pub burn: DriftPayout,
}

/// The drift physics of the car in `bundle`.
pub fn read_drift_physics(bundle: &Bundle) -> Result<DriftPhysics> {
    let types = TypeNames::read(bundle)?;
    let handling = find_object(bundle, &types, "Genesys.Gen.HandlingBehaviour")?;
    let trigger = at(&handling, "Genesys.Gen.DriftBehaviour.Drift_trigger")?;
    let exit = at(&handling, "Genesys.Gen.DriftBehaviour.Drift_exit")?;
    let physics = DriftPhysics {
        trigger_speed: handling.f32(trigger + 0x10)?,
        trigger_angle: handling.f32(trigger + 0x18)?,
        exit_angle: handling.f32(exit + 0x10)?,
        exit_hold: handling.f32(exit + 0x24)?,
    };
    let numbers = [physics.trigger_speed, physics.trigger_angle, physics.exit_angle, physics.exit_hold];
    if !numbers.iter().all(|value| value.is_finite() && *value >= 0.0) || physics.trigger_angle <= physics.exit_angle {
        return Err(Error::Spec(format!("the drift physics do not read sensibly: {physics:?}")));
    }
    Ok(physics)
}

/// The drift payout of the stock and the burn nitrous of the car in `bundle`.
pub fn read_drift_nitrous(bundle: &Bundle) -> Result<DriftNitrous> {
    let types = TypeNames::read(bundle)?;
    let gameplay = find_resource(bundle, &types, "Genesys.Gen.Vehicle_Gameplay")?;
    let payout = |slot: u32| -> Result<DriftPayout> {
        let import = gameplay
            .imports
            .iter()
            .find(|import| import.offset & HANDLE_FLAG != 0 && import.offset & !HANDLE_FLAG == slot)
            .ok_or_else(|| Error::Spec(format!("Vehicle_Gameplay has no nitrous object at +0x{slot:X}")))?;
        let resource = bundle.resource_by_id(import.id)?;
        let object = object_of(resource, &types);
        read_payout(&object)
    };
    Ok(DriftNitrous { stock: payout(STOCK_SLOT)?, burn: payout(BURN_SLOT)? })
}

fn read_payout(object: &Object) -> Result<DriftPayout> {
    let drifting = at(object, "Genesys.Gen.NitrousParameters.Drifting")?;
    let payout = DriftPayout {
        message: object.u32(drifting + 0xC)?,
        scale: object.f32(drifting + 0x10)?,
        min_speed_mph: object.f32(drifting + 0x14)?,
        gain: object.f32(drifting + 0x18)?,
        delay: object.f32(drifting + 0x1C)?,
        flag: object.u32(drifting + 0x20)? != 0,
    };
    let numbers = [payout.scale, payout.min_speed_mph, payout.gain, payout.delay];
    if !numbers.iter().all(|value| value.is_finite() && *value >= 0.0) {
        return Err(Error::Spec(format!("the drift payout does not read sensibly: {payout:?}")));
    }
    Ok(payout)
}

fn at(object: &Object, name: &str) -> Result<usize> {
    object.at(name, 0).ok_or_else(|| Error::Spec(format!("no {name} in the object")))
}

/// The first Genesys object of the bundle that holds a structure of type `marker`, with its imports.
fn find_resource(bundle: &Bundle, types: &TypeNames, marker: &str) -> Result<Resource> {
    let kind = ResourceKind::GenesysObject.id();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind || entry.import_count == 0 {
            continue;
        }
        let resource = bundle.resource(index)?;
        let holds = resource.imports.iter().any(|import| types.name(import.id) == Some(marker));
        if holds {
            return Ok(resource);
        }
    }
    Err(Error::Spec(format!("the bundle has no Genesys object with a {marker}")))
}

/// The structures of an object, from its type imports.
fn object_of(resource: Resource, types: &TypeNames) -> Object {
    let mut structs: Vec<(String, usize)> = resource
        .imports
        .iter()
        .filter(|import| import.offset & HANDLE_FLAG != 0)
        .filter_map(|import| Some((types.name(import.id)?.to_owned(), (import.offset & !HANDLE_FLAG) as usize)))
        .collect();
    structs.sort_by_key(|(_, offset)| *offset);
    let [main, ..] = resource.blocks;
    Object { main, structs }
}
