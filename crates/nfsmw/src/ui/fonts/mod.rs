//! Custom fonts: FEng fonts a player supplies in a folder, chosen per resolution tier. The rules are in
//! `docs/specs/custom-fonts.md`. The files are read by the settings' folder (`settings/custom_fonts.rs`); the font
//! and texture parsers are the shared ones.

mod folder;
mod set;
pub mod tier;

pub use folder::scan;
pub use set::CustomFontSet;
