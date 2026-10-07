//! AttribSys gameplay databases of EA Black Box games: `VPAK` packs of vaults, each a `.vlt`
//! (structure) and `.bin` (payload) blob pair. Spec: `docs/formats/attributes.md`.
//!
//! A database holds **classes** (schemas: typed fields) and **collections** (instances, keyed by
//! name hash, inheriting unset values from a parent collection). Everything is keyed by
//! [`vlt_hash`] (Jenkins lookup2, initval `0xABCDEF00`).
//!
//! ```no_run
//! # fn main() -> blackbox_attrib::Result<()> {
//! # let (attributes_bin, gameplay_bin) = (Vec::new(), Vec::new());
//! use blackbox_attrib::Database;
//!
//! let mut db = Database::open(&attributes_bin)?; // class definitions first
//! db.load(&gameplay_bin)?; // more collections of those classes
//!
//! let car = db.collection("pvehicle", "bmwm3gtr").expect("collection");
//! let mass = car.get_f32("MASS");
//! let chassis = car.follow("chassis"); // a RefSpec, resolved to another collection
//! for c in db.collections_of("tires") {
//!     println!("{}", db.names().display(c.key()));
//! }
//! # Ok(()) }
//! ```
//!
//! Record layouts differ between AttribSys generations; they live in [`layout`] and are detected
//! per vault. Supported today: the legacy layout (NFS: Most Wanted, 2005). Others fail with
//! [`Error::UnsupportedLayout`] until their layout is added.
//!
//! This crate never touches the filesystem: it takes bytes.

mod bytes;
mod class;
mod collection;
mod database;
mod error;
pub mod hash;
pub mod layout;
mod names;
pub mod pack;
mod value;
pub mod vault;

#[cfg(test)]
mod tests;

pub use class::{Class, Field, FieldFlags};
pub use collection::{Attribute, Collection, CollectionRef};
pub use database::{Database, TypeInfo};
pub use error::{Error, Result};
pub use hash::vlt_hash;
pub use names::Names;
pub use value::{Blob, RefSpec, StringKey, TypeKind, Value};
pub use vault::Vault;
