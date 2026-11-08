//! Vehicle components: a base vehicle plus installed components.
//!
//! A [`VehicleDef`] says which slots a vehicle has and which components each slot accepts; a [`Catalog`] holds the
//! components themselves, shared by every vehicle; a [`Selection`] is what the player installed. [`VehicleDef::resolve`]
//! turns the three into a [`ResolvedVehicle`]: the model parts to draw, the surfaces, the parameter modifiers and the
//! behaviours. Nothing here knows a game: parts, surfaces, parameter keys and behaviour domains are opaque strings that
//! the game maps to its solids, materials, physics fields and handlers. Specs: `docs/specs/vehicle-components/`.
//!
//! ```
//! use blackbox_components::{Catalog, Pick, Selection, VehicleDef};
//!
//! let catalog: Catalog = serde_json::from_str(
//!     r#"{"component": [
//!         {"id": "tyre.stock"},
//!         {"id": "tyre.slick", "effects": [{"kind": "param", "key": "grip", "op": "mul", "value": 1.2}]}
//!     ]}"#,
//! )
//! .unwrap();
//! let vehicle: VehicleDef = serde_json::from_str(
//!     r#"{"id": "coupe", "slot": [{"id": "tyres", "accepts": ["tyre.stock", "tyre.slick"], "default": "tyre.stock"}]}"#,
//! )
//! .unwrap();
//! vehicle.check(&catalog).unwrap();
//!
//! let mut selection = Selection::default();
//! selection.set("tyres", Pick::of("tyre.slick"));
//! let resolved = vehicle.resolve(&catalog, &selection).unwrap();
//! assert_eq!(resolved.params.apply("grip", 1.0), 1.2);
//! ```

mod check;
mod codec;
mod effect;
mod model;
mod resolve;
mod selection;
mod state;
mod validate;
mod view;

#[cfg(test)]
mod tests;

pub use codec::{MAX_PICKS, WIRE_VERSION, WireError};
pub use effect::{ColourSource, Effect, Input, Op, PartId, Value};
pub use model::{Catalog, Component, ComponentId, Condition, InputDef, Offer, Slot, SlotId, Used, VehicleDef};
pub use resolve::{Installed, Modifier, Note, ParamTable, ResolvedVehicle, SurfaceState};
pub use selection::{Change, Pick, Selection};
pub use validate::{Gate, Problem};
pub use view::{Line, OptionState, OptionView, Quote, SlotView};
