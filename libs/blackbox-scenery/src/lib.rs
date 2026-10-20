//! Scenery sections from EA Black Box game worlds: which models a world section
//! uses ([`SceneryInfo`]) and where each copy goes ([`SceneryInstance`]).
//! Spec: `docs/formats/maps.md` ("Scenery").
//!
//! The records have no version field, so the caller picks the [`layout`] for its game.

pub mod build;
#[cfg(test)]
mod build_tests;
mod error;
mod group;
#[cfg(test)]
mod group_tests;
mod hierarchy;
mod info;
mod instance;
pub mod layout;
mod lod;
mod override_info;
mod section;
mod spawner;

pub use error::{Error, Result};
pub use group::{SceneryGroup, find_group, override_group_numbers, parse_groups, read_scenery_groups};
pub use hierarchy::{ModelHierarchy, ModelNode, read_model_hierarchies};
pub use info::SceneryInfo;
pub use instance::SceneryInstance;
pub use lod::{LodModel, LodObject, LodPolicy, LodView};
pub use override_info::{
    INSTANCE_TURNABLE, OVERRIDE_HIDDEN, OVERRIDE_RECORD_LEN, OVERRIDE_TURNED, SceneryOverride, parse_overrides,
    read_scenery_overrides,
};
pub use section::{OverrideHook, ScenerySection, read_scenery_sections};
pub use spawner::{ObjectSpawner, read_object_spawners};
