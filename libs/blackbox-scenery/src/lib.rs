//! Scenery sections from EA Black Box game worlds: which models a world section
//! uses ([`SceneryInfo`]) and where each copy goes ([`SceneryInstance`]).
//! Spec: `docs/formats/maps.md` ("Scenery").
//!
//! The records have no version field, so the caller picks the [`layout`] for its game.

mod error;
mod info;
mod instance;
pub mod layout;
mod lod;
mod section;

pub use error::{Error, Result};
pub use info::SceneryInfo;
pub use instance::SceneryInstance;
pub use lod::{LodModel, LodView};
pub use section::{ScenerySection, read_scenery_sections};
