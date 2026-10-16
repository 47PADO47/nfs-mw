//! `bb.audio` on the host side: the backend the game implements, and the trampolines behind `BbAudio`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, name_of, slot_of};
use crate::abi::{BB_ERR_ARG, BbAudio, BbIface, BbStatus, BbStr, IFACE_AUDIO, IFACE_AUDIO_VERSION};

/// What the game implements for `bb.audio`. `owner` is the id of the mod that made the call.
pub trait AudioBackend {
    /// Play event `event` of bank `bank` once at `volume` (0 to 1, already checked). Returns the sound's handle.
    fn play(&self, owner: &str, bank: &str, event: &str, volume: f32) -> BackendResult<u64>;
}

/// Build the `bb.audio` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn AudioBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbAudio {
        head: BbIface { size: size_of::<BbAudio>() as u32, major: IFACE_AUDIO_VERSION.0, minor: IFACE_AUDIO_VERSION.1 },
        ctx: Slot::ctx(&slot),
        play: Some(play),
    });
    let kept = slot.clone();
    let release = Box::new(move || kept.release_with(|_, _, _| {}));
    Attached::new(IFACE_AUDIO, IFACE_AUDIO_VERSION, table, release)
}

/// `play`: a bank and an event name (each at most a name long), and a volume from 0 to 1.
unsafe extern "C" fn play(ctx: *mut c_void, bank: BbStr, event: BbStr, volume: f32, out_id: *mut u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn AudioBackend>(ctx)? };
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        // SAFETY: the mod's names are valid for the call (ADR 0005 section 4).
        let bank = unsafe { name_of(bank) }?;
        // SAFETY: as above.
        let event = unsafe { name_of(event) }?;
        if !(0.0..=1.0).contains(&volume) {
            return Err(BB_ERR_ARG);
        }
        let handle = slot.backend.play(&slot.owner, bank, event, volume)?;
        // SAFETY: `out_id` is non-null (checked above) and valid for one write (the mod's contract).
        unsafe { out_id.write(handle) };
        Ok(())
    })
}
