//! `bb.hud`: a centred message for a few seconds, and hiding it. The message is queued as a [`WorldRequest`] and drawn by the
//! world's `ui` while a world is up (`hud/message.rs`).

use std::rc::Rc;

use blackbox_modding::host::{BackendResult, HudBackend, MessageStyle};

use super::bridge::Bridge;
use super::world::WorldRequest;
use crate::hud::ModMessageTone;

/// The `bb.hud` port of the game: one per mod table, sharing the bridge.
pub struct HudPort(pub(super) Rc<Bridge>);

/// The tone a message style is drawn in.
fn tone_of(style: MessageStyle) -> ModMessageTone {
    match style {
        MessageStyle::Info => ModMessageTone::Info,
        MessageStyle::Warning => ModMessageTone::Warning,
        MessageStyle::Big => ModMessageTone::Big,
    }
}

impl HudBackend for HudPort {
    fn message(&self, owner: &str, text: &str, style: MessageStyle, secs: f32) -> BackendResult<()> {
        let request = WorldRequest::Message { text: crate::i18n::mod_label(owner, text), tone: tone_of(style), secs };
        self.0.world_requests.borrow_mut().push(request);
        Ok(())
    }

    fn hide(&self, _owner: &str) -> BackendResult<()> {
        self.0.world_requests.borrow_mut().push(WorldRequest::HideMessage);
        Ok(())
    }
}
