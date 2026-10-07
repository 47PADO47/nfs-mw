//! The track streaming index (`TrackStreamingSections`) of EA Black Box games:
//! which part of the big stream file holds each section of the world, and where
//! the section sits on the map. Spec: `docs/formats/maps.md`.
//!
//! The record has no version field, so the caller picks the [`layout`] for its game.

mod error;
pub mod layout;
mod reader;
mod section;

pub use error::{Error, Result};
pub use reader::read_sections;
pub use section::StreamingSection;
