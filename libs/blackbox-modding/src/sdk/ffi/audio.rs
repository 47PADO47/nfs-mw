//! `bb.audio` on the mod side: read the table, play a sound.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{BbAudio, BbAudioPlayFn, BbStr};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.audio` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct AudioFns {
    ctx: *mut c_void,
    play: Option<BbAudioPlayFn>,
}

/// Read the `bb.audio` table the host offered.
pub(crate) fn read(iface: Interface) -> AudioFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        AudioFns {
            ctx: read_or(&iface, offset_of!(BbAudio, ctx), null_mut()),
            play: read_or(&iface, offset_of!(BbAudio, play), None),
        }
    }
}

/// Play event `event` of bank `bank` once; returns the raw handle.
pub(crate) fn play(fns: &AudioFns, bank: &str, event: &str, volume: f32) -> ModResult<u64> {
    let play = fns.play.ok_or(Error::Unsupported)?;
    let mut id = 0_u64;
    // SAFETY: the names live for the call; `id` is valid for one write.
    status_result(unsafe { play(fns.ctx, BbStr::new(bank), BbStr::new(event), volume, &mut id) })?;
    Ok(id)
}
