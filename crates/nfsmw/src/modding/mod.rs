//! Native mods (ADR 0005): the game's side of the mod host.
//!
//! The loader and the dispatcher live in `blackbox-modding` (feature `host`). This module decides which mods may load
//! ([`policy`], from the `mods` setting and `--no-mods`), starts them and adds their `files/` folders to the install
//! ([`open_install`]), sends the frame events ([`ModsPlugin`]), and answers the `mods` console command ([`listing`]).
//! The game offers no interface to the mods yet, so they get the log and the events only.

mod console;
mod events;
mod layers;
mod open;
mod plugin;
mod policy;
mod runtime;

#[cfg(test)]
mod hello_tests;
#[cfg(test)]
mod tests;

pub use console::listing;
pub use open::open_install;
pub use plugin::ModsPlugin;
pub use runtime::Mods;
