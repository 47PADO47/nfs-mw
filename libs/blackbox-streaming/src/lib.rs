//! Track streaming data of EA Black Box games:
//!
//! - the streaming index (`TrackStreamingSections`): which part of the big stream
//!   file holds each section of the world, and where the section sits on the map;
//! - the visible-section tables (`VisibleSectionManager`): the map's zones and
//!   which sections each zone loads and draws ([`visible`]).
//!
//! Specs: `docs/formats/maps.md`, `docs/specs/visible-sections.md`. The records
//! have no version field, so the caller picks the [`layout`] for its game.

mod error;
pub mod layout;
mod reader;
mod section;
pub mod visible;

pub use error::{Error, Result};
pub use reader::read_sections;
pub use section::StreamingSection;
pub use visible::{Numbering, VisibleSections, read_visible_sections};
