//! `bb.hud` on the host side: the backend the game implements, and the trampolines behind `BbHud`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, slot_of, text_within};
use crate::abi::{
    BB_ERR_ARG, BB_HUD_BIG, BB_HUD_INFO, BB_HUD_WARNING, BbHud, BbIface, BbStatus, BbStr, IFACE_HUD, IFACE_HUD_VERSION,
    MAX_MESSAGE_BYTES,
};

/// The longest a message stays up, seconds. A longer time is refused.
pub const MAX_MESSAGE_SECS: f32 = 3600.0;

/// How a message is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStyle {
    /// A plain message.
    Info,
    /// A warning, in the warning colour.
    Warning,
    /// A large message.
    Big,
}

impl MessageStyle {
    /// The style a `BB_HUD_*` number names. `None` for an unknown number.
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            BB_HUD_INFO => Some(MessageStyle::Info),
            BB_HUD_WARNING => Some(MessageStyle::Warning),
            BB_HUD_BIG => Some(MessageStyle::Big),
            _ => None,
        }
    }
}

/// What the game implements for `bb.hud`. `owner` is the id of the mod that made the call.
pub trait HudBackend {
    /// Show `text` for `secs` seconds. A newer message replaces the one showing. Already checked: the text is valid
    /// UTF-8 of at most [`MAX_MESSAGE_BYTES`], and `secs` is finite, above zero and at most [`MAX_MESSAGE_SECS`].
    fn message(&self, owner: &str, text: &str, style: MessageStyle, secs: f32) -> BackendResult<()>;
    /// Take the message down now.
    fn hide(&self, owner: &str) -> BackendResult<()>;
}

/// Build the `bb.hud` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn HudBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbHud {
        head: BbIface { size: size_of::<BbHud>() as u32, major: IFACE_HUD_VERSION.0, minor: IFACE_HUD_VERSION.1 },
        ctx: Slot::ctx(&slot),
        message: Some(message),
        hide: Some(hide),
    });
    let kept = slot.clone();
    let release = Box::new(move || kept.release_with(|_, _, _| {}));
    Attached::new(IFACE_HUD, IFACE_HUD_VERSION, table, release)
}

/// `message`: check the text, the style and the time, then show it.
unsafe extern "C" fn message(ctx: *mut c_void, text: BbStr, style: u32, secs: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn HudBackend>(ctx)? };
        // SAFETY: the mod's text is valid for the call (ADR 0005 section 4).
        let text = unsafe { text_within(text, MAX_MESSAGE_BYTES) }?;
        let style = MessageStyle::from_raw(style).ok_or(BB_ERR_ARG)?;
        if !(secs > 0.0 && secs <= MAX_MESSAGE_SECS) {
            return Err(BB_ERR_ARG);
        }
        slot.backend.message(&slot.owner, text, style, secs)
    })
}

/// `hide`: take the message down.
unsafe extern "C" fn hide(ctx: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn HudBackend>(ctx)? };
        slot.backend.hide(&slot.owner)
    })
}
