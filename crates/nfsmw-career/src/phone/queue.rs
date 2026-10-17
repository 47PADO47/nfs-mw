//! Message delivery: `EAddSMS`, the pending list, the per-update flush and the `ECellCall` request
//! (`docs/specs/career-phone.md` sections 3.1 to 3.5).

use super::ids::{is_immediate, is_story};
use super::voice::{VoiceTake, voice_take};
use crate::settings::SmsInbox;

/// What the game state allows when a queued message wants to appear (`career-phone.md` 3.5).
///
/// The flush runs only when all five hold. The caller maps the original conditions onto them:
/// the global game state is "in game" with the disabling globals clear, the race status is idle, the front end
/// can open an overlay (no photo mode, no fade screen), the `cellcall` speech event is not queued or playing,
/// and the player car exists, is not wrecked or loading and the pursuit check on it returns zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlushContext {
    /// The game is in the world (global state 6) and the disabling globals are zero.
    pub free_roam: bool,
    /// The race-status object exists and its state is 0 (not racing).
    pub race_idle: bool,
    /// The front end may open an overlay.
    pub overlay_allowed: bool,
    /// No `cellcall` speech is queued or playing.
    pub speech_idle: bool,
    /// The player car exists, is not wrecked or loading, and the pursuit check is zero.
    pub player_car_ok: bool,
}

impl FlushContext {
    /// A context where everything allows a message.
    pub const ALL_CLEAR: FlushContext = FlushContext {
        free_roam: true,
        race_idle: true,
        overlay_allowed: true,
        speech_idle: true,
        player_car_ok: true,
    };

    /// The "can show" test: all five conditions.
    pub const fn can_show(&self) -> bool {
        self.free_roam && self.race_idle && self.overlay_allowed && self.speech_idle && self.player_car_ok
    }
}

/// What [`PhoneQueue::add_sms`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddOutcome {
    /// An immediate id (10 to 14, 95, 96) was delivered at once; the caller must process the returned
    /// `ECellCall(id)` request ([`process_cell_call`]).
    Delivered {
        /// The call to post.
        call: CellCallRequest,
    },
    /// The id was appended to the pending list.
    Queued,
    /// The id is already waiting in the pending list.
    AlreadyPending,
    /// Nothing happened: the slot is already delivered or read (a repeated trigger cannot duplicate a message),
    /// or an immediate id whose slot is missing or not empty.
    Ignored,
}

/// An `ECellCall(id)` request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellCallRequest {
    /// The call id.
    pub id: u32,
}

/// What a flush did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlushOutcome {
    /// The ids whose slots were delivered, oldest first (sequence numbers follow this order).
    pub delivered: Vec<u32>,
    /// The id the inbox opens at: the first story message of the batch, else the oldest queued id.
    pub open_at: u32,
    /// The HUD engage object gets its flag set (always, when something is opened).
    pub engage_flag: bool,
}

/// The pending list of queued text messages. It is not part of the profile slot table (it is saved in the
/// gameplay blob, which this slice does not decode).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PhoneQueue {
    pending: Vec<u32>,
}

impl PhoneQueue {
    /// An empty queue.
    pub fn new() -> PhoneQueue {
        PhoneQueue::default()
    }

    /// The queued ids, oldest first.
    pub fn pending(&self) -> &[u32] {
        &self.pending
    }

    /// `EAddSMS(id)`:
    ///
    /// - the immediate ids are delivered at once ([`SmsInbox::deliver`]) and answer with the call to post;
    /// - any other id is appended to the pending list unless its slot is already delivered or read, or the id is
    ///   already pending. An id with no slot (150 and above) is queued like the spec says; the flush skips it.
    pub fn add_sms(&mut self, inbox: &mut SmsInbox, id: u32) -> AddOutcome {
        if is_immediate(id) {
            return match inbox.deliver(id) {
                true => AddOutcome::Delivered { call: CellCallRequest { id } },
                false => AddOutcome::Ignored,
            };
        }
        if inbox.slot(id).is_some_and(|slot| slot.state != 0) {
            return AddOutcome::Ignored;
        }
        if self.pending.contains(&id) {
            return AddOutcome::AlreadyPending;
        }
        self.pending.push(id);
        AddOutcome::Queued
    }

    /// The per-update flush. Does nothing while the list is empty or [`FlushContext::can_show`] is false (a
    /// queued message waits with no timeout). Otherwise, for every queued id oldest first, whose slot exists:
    /// the first story message becomes the one to open, the slot goes to state 2 and takes the next sequence
    /// number. Then the list is cleared. When no story message was chosen, the oldest queued id is opened.
    pub fn flush(&mut self, inbox: &mut SmsInbox, ctx: &FlushContext) -> Option<FlushOutcome> {
        if self.pending.is_empty() || !ctx.can_show() {
            return None;
        }
        let pending = std::mem::take(&mut self.pending);
        let mut chosen = None;
        let mut delivered = Vec::new();
        for &id in &pending {
            if inbox.slot(id).is_none() {
                continue;
            }
            if chosen.is_none() && is_story(id) {
                chosen = Some(id);
            }
            inbox.mark_delivered(id);
            delivered.push(id);
        }
        let open_at = chosen.unwrap_or(pending[0]);
        Some(FlushOutcome { delivered, open_at, engage_flag: true })
    }
}

/// The language of the running game: a text language shows a call as an inbox message, a voice language speaks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// English, French, German, Italian or Spanish: calls are spoken.
    Voice,
    /// Dutch, Swedish, Danish, Polish or Finnish: calls are shown as messages.
    Text,
}

impl Language {
    /// The kind of a language id: Dutch (5), Swedish (6), Danish (7), Polish (12) and Finnish (13) are text
    /// languages; English 0, French 1, German 2, Italian 3 and Spanish 4 (and anything else) are voice.
    pub const fn from_language_id(id: u32) -> Language {
        match id {
            5 | 6 | 7 | 12 | 13 => Language::Text,
            _ => Language::Voice,
        }
    }
}

/// What processing an `ECellCall` request did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellCallOutcome {
    /// Text language: the slot (if it exists) is set to state 2 with no sequence number, and the inbox opens at
    /// the slot (`Some(id)`) or at the list (`None`, for a negative id), unless it is already open (the caller
    /// checks that).
    OpenInbox {
        /// The slot to open at.
        at: Option<u32>,
    },
    /// Text language, but the front end cannot open now: the call is lost and the slot stays as it was.
    Lost,
    /// Voice language: speak this take. The inbox is untouched.
    Speak(VoiceTake),
    /// Voice language, id outside 10 to 105: the speech queue is flushed and nothing is said.
    Silent,
}

/// Executes an `ECellCall(id)` request. `id` may be negative (open the list); `overlay_allowed` is the second
/// test of 3.5 (the front end may open an overlay), used in text languages only.
pub fn process_cell_call(inbox: &mut SmsInbox, id: i32, language: Language, overlay_allowed: bool) -> CellCallOutcome {
    if language == Language::Voice {
        return match u32::try_from(id).ok().and_then(voice_take) {
            Some(take) => CellCallOutcome::Speak(take),
            None => CellCallOutcome::Silent,
        };
    }
    if !overlay_allowed {
        return CellCallOutcome::Lost;
    }
    let at = u32::try_from(id).ok();
    if let Some(slot) = at.and_then(|id| inbox.slot_mut(id)) {
        slot.state = 2;
    }
    CellCallOutcome::OpenInbox { at }
}

#[cfg(test)]
mod tests;
