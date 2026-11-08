//! The abstract menu model the option pages are built on: an [`Entry`] that can be a section title, an action or a
//! plain row, the [`Section`]s that group rows under a title, and the navigation rules that skip what a cursor must
//! not land on ([`nav`]). Pure data and rules: no drawing, no input device, no game names. A host maps its own rows onto
//! [`EntryFlags`], and a theme reads the flags to draw (centred accent text for a title, a plate for the selected row).
//!
//! Spec: `docs/specs/menu-sections.md`.

mod entry;
pub mod nav;
mod section;

pub use entry::{Callback, Entry, EntryFlags};
pub use section::{Section, SectionSet, flatten};
