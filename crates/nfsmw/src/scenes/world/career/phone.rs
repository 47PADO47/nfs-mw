//! The phone in free roam: the queued SMS and calls (`docs/specs/career-phone.md` section 7).
//!
//! Messages wait in a [`PhoneQueue`] until the world allows them: a bin's messages are queued when the bin is entered
//! ([`PhoneState::queue_bin_messages`]), and `EAddSMS` goes through [`PhoneState::add_sms`]. Once a frame the queue
//! is flushed into the career's inbox when [`FlushContext::can_show`] holds, and the front end is asked to open the
//! message over the game ([`popup_request`], `flow_sms.rs`).

use nfsmw_career::blacklist::{BinState, Blacklist};
use nfsmw_career::phone::{
    AddOutcome, BinMoment, CellCallOutcome, CellCallRequest, FlushContext, FlushOutcome, Language, PhoneQueue,
    bin_messages, process_cell_call,
};
use nfsmw_career::settings::SmsInbox;

use crate::audio::cell_call;
use crate::career::CareerRequest;
use crate::scenes::world::WorldScene;

/// The bins the Blacklist has (1 to 15). Bin 16 is the prologue and has no rival.
const LAST_RIVAL_BIN: u8 = 15;

/// The phone's state between frames.
pub(in crate::scenes::world) struct PhoneState {
    /// Messages waiting for a free moment, oldest first.
    queue: PhoneQueue,
    /// `ECellCall` requests from the immediate ids, processed on the next frame.
    calls: Vec<CellCallRequest>,
    /// The kind of the running game's language: text languages show a call as a message, voice languages speak it.
    /// Not read from the install yet, so it stays at voice.
    language: Language,
    /// The Blacklist as the last frame saw it. `None` until the first frame of the career: that frame only records
    /// the state, so a career loaded mid-progress does not replay the messages of the bins it has already passed.
    seen: Option<BinSnapshot>,
}

impl Default for PhoneState {
    fn default() -> Self {
        PhoneState { queue: PhoneQueue::new(), calls: Vec::new(), language: Language::Voice, seen: None }
    }
}

/// The Blacklist facts the bin messages depend on: the current bin, and which bins have their rival unlocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::scenes::world) struct BinSnapshot {
    /// `CurrentBin`.
    pub current: u8,
    /// Bit `n` is set when bin `n`'s rival is unlocked. A bin at `done` counts: `done` follows `rival_unlocked`.
    pub unlocked: u16,
}

/// Reads the bin facts of the Blacklist.
pub(super) fn bin_snapshot(blacklist: &Blacklist) -> BinSnapshot {
    let mut unlocked = 0u16;
    for number in 1..=LAST_RIVAL_BIN {
        let state = blacklist.bin(number).and_then(|record| record.state);
        if rival_is_unlocked(state) {
            unlocked |= 1 << number;
        }
    }
    BinSnapshot { current: blacklist.current_bin(), unlocked }
}

/// Whether a bin's state has its rival unlocked: `rival_unlocked`, or `done` after it.
fn rival_is_unlocked(state: Option<BinState>) -> bool {
    matches!(state, Some(BinState::RivalUnlocked | BinState::Done))
}

/// The messages that became due between two frames: the entry message of each bin entered (from the one above the
/// seen bin down to the current one, the order the Blacklist enters them), then the rival message of each rival
/// that unlocked. Nothing is due on the first frame (`seen` is `None`).
pub(super) fn due_bin_messages(seen: Option<BinSnapshot>, now: BinSnapshot) -> Vec<u32> {
    let Some(seen) = seen else { return Vec::new() };
    let mut ids = Vec::new();
    for bin in (now.current..seen.current).rev() {
        ids.extend_from_slice(bin_messages(bin, BinMoment::Entry));
    }
    for bin in 1..=LAST_RIVAL_BIN {
        let bit = 1u16 << bin;
        let newly_unlocked = now.unlocked & bit != 0 && seen.unlocked & bit == 0;
        if newly_unlocked {
            ids.extend_from_slice(bin_messages(bin, BinMoment::RivalUnlock));
        }
    }
    ids
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
    /// A race is being run: messages wait for the free roam after it.
    pub racing: bool,
}

/// Maps the world facts onto the five conditions of `career-phone.md` 3.5.
///
/// The race test fails while a race is run. The zone card stands in for "not in a zone
/// card", so it holds the flush back (`free_roam`). The speech queue is not reachable from the world yet, so it
/// always reads idle.
pub(super) fn flush_context(facts: &PhoneFacts) -> FlushContext {
    FlushContext {
        free_roam: !facts.card_up,
        race_idle: !facts.racing,
        overlay_allowed: !facts.overlay_blocked,
        speech_idle: true,
        player_car_ok: facts.car_ready && !facts.pursuing,
    }
}

/// The request a flush makes: the message screen opens at the message the flush chose (the first story message of the
/// batch, else the oldest).
pub(super) fn popup_request(flush: &FlushOutcome) -> CareerRequest {
    CareerRequest::Sms { id: flush.open_at }
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
    /// [`bin_messages`]. Returns how many were newly queued; a message already pending, delivered or read is skipped.
    pub fn queue_bin_messages(&mut self, inbox: &mut SmsInbox, ids: &[u32]) -> usize {
        ids.iter().filter(|&&id| self.add_sms(inbox, id) == AddOutcome::Queued).count()
    }

    /// Queues the messages that became due since the last frame (see [`due_bin_messages`]) and records `now` as seen.
    pub fn note_bins(&mut self, inbox: &mut SmsInbox, now: BinSnapshot) {
        let due = due_bin_messages(self.seen, now);
        self.seen = Some(now);
        self.queue_bin_messages(inbox, &due);
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
    /// Queues the bin messages that came due (a bin entered or a rival unlocked since the last frame), then flushes
    /// the phone queue into the career's inbox when free roam allows it, and asks the front end to open what arrived.
    /// Calls waiting from the immediate ids are processed first.
    pub(super) fn career_phone(&mut self, _dt: f32) {
        let facts = self.phone_facts();
        let ctx = flush_context(&facts);
        let phone = &mut self.career.phone;
        let flushed = self.career.handle.with(|career| {
            let now = bin_snapshot(&career.blacklist);
            let inbox = &mut career.settings.sms;
            phone.note_bins(inbox, now);
            for call in phone.take_calls() {
                // Ids from the calls are small positive numbers, so the `i32` cast is exact.
                let outcome = process_cell_call(inbox, call.id as i32, phone.language, ctx.overlay_allowed);
                log_call(call.id, outcome);
                if let CellCallOutcome::Speak(_) = outcome {
                    cell_call::request(call.id);
                }
            }
            phone.flush(inbox, &ctx)
        });
        let Some(flush) = flushed.flatten() else { return };
        log::info!("phone: {} message(s) delivered, the message {} opens", flush.delivered.len(), flush.open_at);
        self.career.requests.push_back(popup_request(&flush));
    }

    /// The world facts the flush test reads this frame.
    fn phone_facts(&self) -> PhoneFacts {
        PhoneFacts {
            pursuing: self.traffic.as_ref().is_some_and(|traffic| traffic.is_pursuing()),
            overlay_blocked: self.garage_open,
            card_up: self.entry.popup.is_up(),
            car_ready: self.drive.is_some() && self.pending_car.is_none() && !self.wait_for_load,
            racing: self.race.is_some(),
        }
    }
}

/// Logs what an `ECellCall` request did. A voice call is said by the audio plugin (`cell_call.rs`); the text-language
/// outcomes wait for the text-language flow.
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
