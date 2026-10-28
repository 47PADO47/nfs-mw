//! The `[bike]` table of a RIDE 6 bike, converted from the vehicle's resolved physics data.
//!
//! The input is the JSON the physics dump writes for one vehicle ([`vehicle::Vehicle`]; `docs/formats/ride6-physics-data.md`):
//! its properties resolved over the class defaults, and every asset it links (tyre grip tables, torque curve, the
//! controllers). The output is the text of the `[bike]` table of `cars/<id>.toml` (`docs/modding/bikes.md`) in SI units. Every
//! number is a number of that data converted by the factors of [`units`], a constructor default a spec of this repository
//! names ([`defaults`]) or one of the few modelling choices listed in `docs/modding/ride6-bikes.md`. The mapping of each field
//! is `docs/specs/vehicle-two-wheel-parameters.md`, section 1.
//!
//! What the meshes give (the wheels' radius and width, the hubs and the body box) is passed in as [`Geometry`].

pub mod brakes;
pub mod chassis;
pub mod curve;
pub mod damping;
pub mod defaults;
pub mod engine;
pub mod node;
pub mod rider_table;
pub mod steering;
pub mod suspension;
pub mod tcs;
pub mod thermal;
pub mod toml_text;
pub mod transmission;
pub mod units;
pub mod vehicle;
pub mod wheel;
pub mod wheelie;
pub mod wheelie_help;

use anyhow::Result;

use toml_text::Toml;
use vehicle::Vehicle;

/// The two wheels of a bike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Front,
    Rear,
}

impl Side {
    /// The word of the manifest's tables (`bike.front`, `bike.front_suspension`).
    pub fn name(self) -> &'static str {
        match self {
            Side::Front => "front",
            Side::Rear => "rear",
        }
    }

    pub fn wheel_key(self) -> &'static str {
        match self {
            Side::Front => "FrontWheel",
            Side::Rear => "RearWheel",
        }
    }

    pub fn suspension_key(self) -> &'static str {
        match self {
            Side::Front => "FrontSuspension",
            Side::Rear => "RearSuspension",
        }
    }
}

/// A tyre as the mesh has it: the hub in the body solid's frame (x forward, y left, z up) and the tyre's size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TyreGeometry {
    pub hub: [f64; 3],
    pub radius_m: f64,
    pub width_m: f64,
}

/// What the converted meshes say about the bike.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub front: TyreGeometry,
    pub rear: TyreGeometry,
    /// Half dimensions of the body box (x across, y up, z along), metres.
    pub half_m: [f64; 3],
}

impl Geometry {
    /// Distance between the hubs.
    pub fn wheelbase(&self) -> f64 {
        self.front.hub[0] - self.rear.hub[0]
    }

    pub fn tyre(&self, side: Side) -> &TyreGeometry {
        match side {
            Side::Front => &self.front,
            Side::Rear => &self.rear,
        }
    }
}

/// The text of the `[bike]` table and its sub-tables, and the notes the conversion has for the user.
pub fn bike_table(vehicle: &Vehicle, geometry: &Geometry) -> Result<(String, Vec<String>)> {
    let mut t = Toml::new();
    chassis::write(&mut t, vehicle, geometry)?;
    rider_table::write(&mut t, vehicle)?;
    for side in [Side::Front, Side::Rear] {
        wheel::write(&mut t, vehicle, side, geometry.tyre(side))?;
        thermal::write(&mut t, vehicle, side)?;
    }
    for side in [Side::Front, Side::Rear] {
        suspension::write(&mut t, vehicle, side)?;
    }
    let engine = engine::EngineData::read(vehicle)?;
    engine::write(&mut t, &engine);
    transmission::write(&mut t, vehicle)?;
    brakes::write(&mut t, vehicle)?;
    steering::write(&mut t, vehicle, geometry)?;
    wheelie::write(&mut t, vehicle)?;
    tcs::write(&mut t, vehicle)?;
    Ok(t.into_parts())
}
