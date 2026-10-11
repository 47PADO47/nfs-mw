//! The whole car: every class `pvehicle` links to, in the vehicle library's `VehicleSpec`.

use anyhow::{Context, Result};
use blackbox_attrib::{Database, Value};
use blackbox_vehicle::VehicleSpec;
use blackbox_vehicle::nos::NosSpec;

use super::bounds::CarBounds;
use super::fields::Fields;
use super::{WallSpec, aero, body, brakes, chassis, engine, induction, nos, tires, transmission};

/// A car ready for the physics: its spec and where the collision box sits in the car's own space.
#[derive(Debug, Clone)]
pub struct CarPhysics {
    pub spec: VehicleSpec,
    /// The collision box and where its centre is relative to the car model's origin (physics axes).
    pub bounds: CarBounds,
    /// How the car reacts to walls.
    pub walls: WallSpec,
}

/// The physics of car type `type_name` (e.g. `BMWM3GTR`), read from `db` (`attributes.bin`).
///
/// Every class is the stock (first) collection `pvehicle` links to, except the nitrous: a stock car
/// often has none, so the first level that has any is used, so the nitrous key does something.
pub fn car_physics(db: &Database, type_name: &str, bounds: CarBounds) -> Result<CarPhysics> {
    let name = type_name.to_ascii_lowercase();
    let pvehicle = db.collection("pvehicle", &name).with_context(|| format!("no vehicle record named {name}"))?;
    let link = |field: &str| {
        pvehicle.follow(field).map(Fields).with_context(|| format!("{name}: the {field} link is missing"))
    };
    let body = body(Fields(pvehicle));
    let walls = body.walls;
    let chassis_fields = link("chassis")?;
    let spec = VehicleSpec {
        mass: body.mass,
        dimension: bounds.half_dimensions,
        tensor_scale: body.tensor_scale,
        body: body.spec,
        chassis: chassis(chassis_fields),
        tires: tires(link("tires")?),
        brakes: brakes(link("brakes")?),
        engine: engine(link("engine")?),
        transmission: transmission(link("transmission")?),
        // Cars without an induction link (the traffic cars) are naturally aspirated.
        induction: pvehicle.follow("induction").map(Fields).map(induction).unwrap_or_default(),
        nos: first_nitrous(db, Fields(pvehicle)),
        aero: aero(chassis_fields),
    };
    Ok(CarPhysics { spec, bounds, walls })
}

/// The first nitrous level of the car that has a tank (the stock level when it has one).
fn first_nitrous(db: &Database, pvehicle: Fields<'_>) -> NosSpec {
    let levels = match pvehicle.0.get("nos") {
        Some(Value::Array(items)) => items.as_slice(),
        _ => &[],
    };
    levels
        .iter()
        .filter_map(Value::as_ref_spec)
        .filter_map(|r| db.resolve(r))
        .map(|c| nos(Fields(c)))
        .find(NosSpec::present)
        .unwrap_or_else(|| nos(Fields(db.collection("nos", "default").unwrap_or(pvehicle.0))))
}
