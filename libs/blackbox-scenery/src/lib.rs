//! Scenery sections from EA Black Box game worlds: which models a world section
//! uses ([`SceneryInfo`]) and where each copy goes ([`SceneryInstance`]).
//! Spec: `docs/formats/maps.md` ("Scenery").
//!
//! The records have no version field, so the caller picks the [`layout`] for its game.

pub mod build;
#[cfg(test)]
mod build_tests;
mod error;
mod hierarchy;
mod info;
mod instance;
pub mod layout;
mod lod;
mod section;
mod spawner;

pub use error::{Error, Result};
pub use hierarchy::{ModelHierarchy, ModelNode, read_model_hierarchies};
pub use info::SceneryInfo;
pub use instance::SceneryInstance;
pub use lod::{LodModel, LodView};
pub use section::{OverrideHook, ScenerySection, read_scenery_sections};
pub use spawner::{ObjectSpawner, read_object_spawners};
