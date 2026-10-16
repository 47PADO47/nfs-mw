//! Round trips in one process: the game's backend is a fake, the table is the one the host builds, and the SDK wrappers read it
//! the way a mod does. No cdylib, no game, no Bevy.

#![allow(unsafe_code)] // The tests call the tables through the same `unsafe` path a mod takes.

mod career;
mod events;
mod fake;
mod pursuit;

use core::mem::size_of;

use blackbox_modding::abi::BbIface;

use crate::abi::{BbNfsmwCareer, BbNfsmwPursuit, BbNfsmwRaces};
use crate::sdk::ffi;
use crate::sdk::{Career, Pursuit, Races};

/// The SDK's view of the pursuit table at `header`, as a mod reads it.
pub(crate) fn pursuit_of(header: *const BbIface) -> Pursuit {
    // SAFETY: the test keeps the host table alive while the wrapper is used, and the size is the table's own.
    Pursuit::new(unsafe { ffi::pursuit::read_raw(header, size_of::<BbNfsmwPursuit>() as u32) })
}

/// The SDK's view of the career table at `header`.
pub(crate) fn career_of(header: *const BbIface) -> Career {
    // SAFETY: as for `pursuit_of`.
    Career::new(unsafe { ffi::career::read_raw(header, size_of::<BbNfsmwCareer>() as u32) })
}

/// The SDK's view of the race table at `header`.
pub(crate) fn races_of(header: *const BbIface) -> Races {
    // SAFETY: as for `pursuit_of`.
    Races::new(unsafe { ffi::races::read_raw(header, size_of::<BbNfsmwRaces>() as u32) })
}
