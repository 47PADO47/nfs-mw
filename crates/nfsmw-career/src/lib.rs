//! NFS: Most Wanted career rules, written from the specs in `docs/` (spec-first, see `docs/licensing.md`).
//!
//! The crate is pure logic: parsers take `&[u8]`, writers return `Vec<u8>`, nothing opens a file and nothing
//! depends on Bevy. It covers the settings and car stable of the profile ([`settings`], [`stable`]), the pursuit,
//! fine and impound rules ([`economy`]), the SMS inbox ([`phone`]), the safe house flow ([`safehouse`]), the rap
//! sheet ([`rapsheet`]), the save screens' autosave, write and load rules ([`savefile`]), the closed regions
//! ([`regions`]) and the glue that reads and rewrites a whole profile file ([`save`]). Milestones, the Blacklist
//! graphs and races are later slices.
//!
//! Every rule cites its spec in the module docs. Where a spec is silent or marked unconfirmed, the code takes
//! the most conservative reading and says so next to it; `docs/provenance/career-core.md` lists those choices.

mod bytes;
pub mod economy;
mod error;
pub mod phone;
pub mod rapsheet;
pub mod regions;
pub mod safehouse;
pub mod save;
pub mod savefile;
pub mod settings;
pub mod stable;

pub use error::LayoutError;
