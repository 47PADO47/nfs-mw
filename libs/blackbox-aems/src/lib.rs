//! AEMS of EA Black Box games: a reader for the module banks inside `.abk` files and an interpreter for the
//! event-sound graphs in them (which samples play, how loud, how high). Layout: `docs/formats/aems.md`;
//! behaviour: `docs/specs/aems.md`.
//!
//! A [`ModuleBank`] parses the bytes of a file. An [`Instance`] is one running module: the game sets its class data
//! (the parameters of the sound object), calls [`Instance::update`] once per tick, and a [`Host`] receives what the
//! graph asks for: voices to start, change and stop, and other objects to create.
//!
//! Pure and deterministic: no files, no audio output, no game knowledge. What a class means, which bank holds the
//! samples and how a voice is played is the caller's.
//!
//! ```
//! use blackbox_aems::{Instance, ModuleBank, NullHost};
//!
//! # fn main() -> Result<(), blackbox_aems::Error> {
//! # let bytes = blackbox_aems::minimal_bank();
//! let bank = ModuleBank::parse(&bytes)?;
//! let mut instance = Instance::new(&bank, 0, 1)?;
//! instance.set_class_data(&[1, 2, 3]);
//! instance.update(1000.0 / 60.0, &mut NullHost)?;
//! # Ok(()) }
//! ```

mod bank;
mod code;
mod error;
mod host;
mod instance;
mod mem;
mod nodes;
mod testbank;

#[cfg(test)]
mod tests;

pub use bank::{Interface, Module, ModuleBank};
pub use error::{Error, Result};
pub use host::{ClassCall, Host, NullHost, PlayerInputs, SampleEntry, VoiceState, input};
pub use instance::{Instance, PlayerView};
pub use testbank::minimal_bank;
