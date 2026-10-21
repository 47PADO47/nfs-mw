//! The whole car: every class `pvehicle` links to, in the vehicle library's `VehicleSpec`.

use anyhow::{Context, Result};
use blackbox_attrib::Database;
use blackbox_vehicle::VehicleSpec;
use std::sync::Arc;

use super::fields::Fields;
use super::{AiCollisionReactions, CarEventSpec, CarGeometry, bounds::CarBounds};
use super::{
    WallSpec, aero, body, brakes, chassis, engine, induction, no_engine, no_transmission, nos, tires, transmission,
};

/// A car ready for the physics: its spec and where the collision box sits in the car's own space.
#[derive(Debug, Clone)]
pub struct CarPhysics {
    pub effect_source: u32,
    pub spec: VehicleSpec,
    pub tire_integration: blackbox_vehicle::tires::TireIntegration,
    pub jump_config: blackbox_vehicle::vehicle::JumpConfig,
    /// The collision box and where its centre is relative to the car model's origin (physics axes).
    pub bounds: CarBounds,
    /// How the car reacts to walls.
    pub walls: WallSpec,
    pub geometry: Option<Arc<CarGeometry>>,
    pub events: Option<Arc<CarEventSpec>>,
    pub reactions: Option<Arc<AiCollisionReactions>>,
}

/// The physics of car type `type_name` (e.g. `BMWM3GTR`), read from `db` (`attributes.bin`).
///
/// Every class is the original-equipment collection `pvehicle` links to. Upgrades are resolved
/// explicitly through `performance::apply`; stock cars do not receive an automatic nitrous kit.
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
        // A trailer has no engine and no gearbox: a body that only rolls and brakes.
        engine: pvehicle.follow("engine").map_or_else(no_engine, |c| engine(Fields(c))),
        transmission: pvehicle.follow("transmission").map_or_else(no_transmission, |c| transmission(Fields(c))),
        // Cars without an induction link (the traffic cars) are naturally aspirated.
        induction: pvehicle.follow("induction").map(Fields).map(induction).unwrap_or_default(),
        nos: pvehicle.follow("nos").map(Fields).map(nos).unwrap_or_default(),
        aero: aero(chassis_fields),
    };
    let reactions = AiCollisionReactions::read(db, pvehicle).map(Arc::new);
    Ok(CarPhysics {
        effect_source: pvehicle.key(),
        spec,
        bounds,
        walls,
        geometry: None,
        events: None,
        reactions,
        tire_integration: blackbox_vehicle::tires::TireIntegration::Reference,
        jump_config: super::jump_config(),
    })
}
