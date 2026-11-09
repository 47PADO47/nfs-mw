//! Which textures of the 2005 world are replaced by what (`docs/modding/road-textures.md`): the pictures of the 2012 game
//! ([`sources`]), the looks made of them ([`recipes`]) and one row for each texture replaced ([`road`], [`ground`], [`covers`],
//! [`signs`]).

mod covers;
mod ground;
pub mod recipes;
mod road;
mod signs;
pub mod sources;

use super::recipe::Entry;

/// Every texture replaced, in the order of the tables.
pub fn entries() -> impl Iterator<Item = &'static Entry> {
    [road::ENTRIES, ground::ENTRIES, covers::ENTRIES, signs::ENTRIES].into_iter().flatten()
}

/// The entry that replaces the texture with name hash `hash`.
pub fn find(hash: u32) -> Option<&'static Entry> {
    entries().find(|entry| entry.hash == hash)
}
