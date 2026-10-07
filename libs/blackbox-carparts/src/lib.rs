//! Car tables of EA Black Box games: car types, the parts database, slot types,
//! preset cars and light materials. Spec: `docs/formats/cardata.md`.
//!
//! ```no_run
//! # fn main() -> blackbox_carparts::Result<()> {
//! # let globalb: Vec<u8> = Vec::new();
//! use blackbox_carparts::{PartQuery, layout::MOST_WANTED};
//!
//! let db = blackbox_carparts::read_parts_db(&globalb, &MOST_WANTED)?;
//! let body = db.find(&PartQuery::new(23, blackbox_hash::bstring_hash("BMWM3GTR")).upgrade_level(0));
//! let solid = body.and_then(|p| db.model_hash(p, 0)); // LOD A
//! # Ok(()) }
//! ```
//!
//! Every reader takes the bytes of a file (already unwrapped) and a game layout from [`layout`].

mod bytes;
mod error;
pub mod layout;
mod materials;
mod parts;
mod presets;
mod slots;
mod types;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use materials::{LightMaterial, read_light_materials};
pub use parts::{Part, PartQuery, PartsDb, read_parts_db};
pub use presets::{PresetRide, read_preset_rides};
pub use slots::{SlotOverride, SlotTypes, read_slot_types};
pub use types::{CarTypeInfo, read_car_types};
