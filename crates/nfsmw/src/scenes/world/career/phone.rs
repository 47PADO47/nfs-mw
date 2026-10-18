//! The phone in free roam: the queued SMS and calls (`docs/specs/career-phone.md` section 7).
//!
//! Messages wait in a [`PhoneQueue`] until the world allows them: a bin's messages are queued when the bin is entered
//! ([`PhoneState::queue_bin_messages`]), and `EAddSMS` goes through [`PhoneState::add_sms`]. Once a frame the queue
//! is flushed into the career's inbox when [`FlushContext::can_show`] holds, and a notice goes on the interact card.

use nfsmw_career::phone::{
    AddOutcome, CellCallOutcome, CellCallRequest, FlushContext, FlushOutcome, Language, PhoneQueue, process_cell_call,
};
use nfsmw_career::settings::SmsInbox;

use crate::hud::Prompt;
use crate::scenes::world::WorldScene;

/// How long the delivery notice holds the interact card, in seconds.
const NOTICE_SECS: f32 = 3.0;

/// The phone's state between frames.
pub(in crate::scenes::world) struct PhoneState {
    /// Messages waiting for a free moment, oldest first.
    queue: PhoneQueue,
    /// `ECellCall` requests from the immediate ids, processed on the next frame.
    calls: Vec<CellCallRequest>,
    /// The kind of the running game's language: text languages show a call as a message, voice languages speak it.
    /// Not read from the install yet, so it stays at voice.
    language: Language,
}

impl Default for PhoneState {
    fn default() -> Self {
        PhoneState { queue: PhoneQueue::new(), calls: Vec::new(), language: Language::Voice }
    }
}

/// The world facts the flush test reads, gathered once a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct PhoneFacts {
    /// A chase is on (`TrafficWorld::is_pursuing`).
    pub pursuing: bool,
    /// The garage or another front-end overlay is up.
    pub overlay_blocked: bool,
    /// The interact card is up: a zone prompt or a notice.
    pub card_up: bool,
    /// The player car exists and is not waiting to load.
    pub car_ready: bool,
}

/// Maps the world facts onto the five conditions of `career-phone.md` 3.5.
///
/// The world only runs in free roam, so the race test always passes. The zone card stands in for "not in a zone
/// card", so it holds the flush back (`free_roam`). The speech queue is not reachable from the world yet, so it
/// always reads idle.
pub(super) fn flush_context(facts: &PhoneFacts) -> FlushContext {
    FlushContext {
        free_roam: !facts.card_up,
        race_idle: true,
        overlay_allowed: !facts.overlay_blocked,
        speech_idle: true,
        player_car_ok: facts.car_ready && !facts.pursuing,
    }
}

/// The notice raised when messages arrive on the interact card. The text is a placeholder until the inbox screen
/// exists in the world.
pub(super) fn delivery_prompt() -> Prompt {
    Prompt { title: "New text message".to_string(), line: "Check the inbox".to_string() }
}

impl PhoneState {
    /// `EAddSMS(id)`: delivers an immediate id at once (its call waits in the call list) or queues the id.
    pub fn add_sms(&mut self, inbox: &mut SmsInbox, id: u32) -> AddOutcome {
        let outcome = self.queue.add_sms(inbox, id);
        if let AddOutcome::Delivered { call } = outcome {
            self.calls.push(call);
        }
        outcome
    }

    /// Queues the messages of a bin: its entry message and the ones its wins and challenges send. The ids come from
    /// the bin data (`BinDef` does not carry them yet). Returns how many were newly queued; a message already
    /// pending, delivered or read is skipped.
    #[allow(dead_code)] // called by the blacklist glue when a bin is entered, not wired yet
    pub fn queue_bin_messages(&mut self, inbox: &mut SmsInbox, ids: &[u32]) -> usize {
        ids.iter().filter(|&&id| self.add_sms(inbox, id) == AddOutcome::Queued).count()
    }

    /// The per-frame flush. Does nothing while the queue is empty or the world does not allow it.
    pub fn flush(&mut self, inbox: &mut SmsInbox, ctx: &FlushContext) -> Option<FlushOutcome> {
        self.queue.flush(inbox, ctx)
    }

    /// The `ECellCall` requests waiting to be processed, oldest first. Empties the list.
    pub fn take_calls(&mut self) -> Vec<CellCallRequest> {
        std::mem::take(&mut self.calls)
    }
}

impl WorldScene {
    /// Flushes the phone queue into the career's inbox when free roam allows it, and raises the notice of what
    /// arrived. Calls waiting from the immediate ids are processed first.
    pub(super) fn career_phone(&mut self, _dt: f32) {
        let facts = self.phone_facts();
        let ctx = flush_context(&facts);
        let phone = &mut self.career.phone;
        let flushed = self.career.handle.with(|career| {
            let inbox = &mut career.settings.sms;
            for call in phone.take_calls() {
                // Ids from the calls are small positive numbers, so the `i32` cast is exact.
                let outcome = process_cell_call(inbox, call.id as i32, phone.language, ctx.overlay_allowed);
                log_call(call.id, outcome);
            }
            phone.flush(inbox, &ctx)
        });
        let Some(flush) = flushed.flatten() else { return };
        log::info!("phone: {} message(s) delivered, inbox would open at {}", flush.delivered.len(), flush.open_at);
        self.entry.popup.flash(delivery_prompt(), NOTICE_SECS);
    }

    /// The world facts the flush test reads this frame.
    fn phone_facts(&self) -> PhoneFacts {
        PhoneFacts {
            pursuing: self.traffic.as_ref().is_some_and(|traffic| traffic.is_pursuing()),
            overlay_blocked: self.garage_open,
            card_up: self.entry.popup.is_up(),
            car_ready: self.drive.is_some() && self.pending_car.is_none() && !self.wait_for_load,
        }
    }
}

/// Logs what an `ECellCall` request did. The speech and the inbox screen are not wired to the world yet.
fn log_call(id: u32, outcome: CellCallOutcome) {
    match outcome {
        CellCallOutcome::OpenInbox { at } => log::info!("phone: call {id} opens the inbox at {at:?}"),
        CellCallOutcome::Lost => log::info!("phone: call {id} lost, an overlay is up"),
        CellCallOutcome::Speak(take) => log::info!("phone: call {id} speaks group {} line {}", take.group, take.line),
        CellCallOutcome::Silent => log::info!("phone: call {id} has no voice"),
    }
}

#[cfg(test)]
#[path = "phone_tests.rs"]
mod tests;
