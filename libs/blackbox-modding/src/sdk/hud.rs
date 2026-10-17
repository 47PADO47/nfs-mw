//! `Hud`: a centred message for a few seconds, and hiding it (catalogue: docs/modding/api-world.md#hud).

use crate::abi::{BB_HUD_BIG, BB_HUD_INFO, BB_HUD_WARNING};

use super::ModResult;
use super::ffi::hud::{self as host_hud, HudFns};

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
    fn raw(self) -> u32 {
        match self {
            MessageStyle::Info => BB_HUD_INFO,
            MessageStyle::Warning => BB_HUD_WARNING,
            MessageStyle::Big => BB_HUD_BIG,
        }
    }
}

/// The `bb.hud` interface of the host, as a mod uses it. Obtained from `Ctx::hud`.
#[derive(Clone, Copy)]
pub struct Hud {
    fns: HudFns,
}

impl Hud {
    pub(crate) fn new(fns: HudFns) -> Self {
        Hud { fns }
    }

    /// Show `text` in `style` for `secs` seconds (above zero, at most an hour). A newer message replaces it.
    pub fn message(&self, text: &str, style: MessageStyle, secs: f32) -> ModResult<()> {
        host_hud::message(&self.fns, text, style.raw(), secs)
    }

    /// Take the message down now.
    pub fn hide(&self) -> ModResult<()> {
        host_hud::hide(&self.fns)
    }
}
