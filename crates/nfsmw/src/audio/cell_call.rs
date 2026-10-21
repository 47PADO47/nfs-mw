//! The voice of a phone call or a text message (`docs/specs/career-phone.md` sections 1.4 and 3.3, `docs/specs/speech.md`).
//!
//! The career and the front end ask for a voice by its message or call id: [`request`] queues it like any speech
//! request, so the dispatcher's rules decide when it says, and [`replay`] queues it to play at once, cutting the line
//! in progress (the Replay button). The request becomes the speech event `cellcall` whose two parameters are the take's
//! group and line as one-bit masks ([`take_params`]). The audio plugin takes the queue every frame with [`service`].
//!
//! Like the one-shot queue, the requests wait in a thread-local list: the career, the front end and the audio plugin all
//! run on the main thread.

use std::cell::RefCell;

use nfsmw_career::phone::{VoiceTake, has_recorded_take, voice_take};

use super::Audio;
use super::speech::{Param, SpeechParams};

/// The speech event that says a phone call.
const EVENT: &str = "cellcall";

/// A voice the career asked for: the message or call id, and whether it cuts the line in progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceRequest {
    /// The message or call id (`SMS_MESSAGE_<id>`).
    pub id: u32,
    /// True for Replay: the voice plays at once and cuts the line that sounds. False for a call or a first reading,
    /// which waits in the speech queue.
    pub cut: bool,
}

thread_local! {
    /// The voices asked for since the audio plugin last took them.
    static QUEUE: RefCell<Vec<VoiceRequest>> = const { RefCell::new(Vec::new()) };
}

/// Queues the voice of message or call `id` for the speech queue.
pub fn request(id: u32) {
    push(VoiceRequest { id, cut: false });
}

/// Queues the voice of message `id` to play at once, cutting the line in progress.
pub fn replay(id: u32) {
    push(VoiceRequest { id, cut: true });
}

fn push(request: VoiceRequest) {
    QUEUE.with(|queue| queue.borrow_mut().push(request));
}

/// Takes the voices queued since the last call, in the order they were asked for.
fn take() -> Vec<VoiceRequest> {
    QUEUE.with(|queue| std::mem::take(&mut *queue.borrow_mut()))
}

/// The recorded take of message or call `id`. Ids outside 10 to 105 have none, and neither does id 16 (its request is
/// still made and plays nothing).
pub fn recorded_take(id: u32) -> Option<VoiceTake> {
    if !has_recorded_take(id) {
        return None;
    }
    voice_take(id)
}

/// The parameters of the `cellcall` event that select `take`: `cell_call_bucket` is `1 << group` and
/// `cell_call_number` is `1 << line` (the masks of [`VoiceTake::masks`]).
pub fn take_params(take: VoiceTake) -> SpeechParams {
    let (bucket, number) = take.masks();
    SpeechParams::new().with(Param::CellCallBucket, bucket).with(Param::CellCallNumber, number)
}

/// Plays the voices queued since the last call. Called by the audio plugin every frame.
pub(super) fn service(audio: &mut Audio) {
    for request in take() {
        audio.play_voice(request);
    }
}

impl Audio {
    /// Hands one voice request to the speech system. A message with no recorded take, or speech switched off, says
    /// nothing; the reason goes to the log.
    fn play_voice(&mut self, request: VoiceRequest) {
        let Some(take) = recorded_take(request.id) else {
            log::info!("voice for {}: no recording in the install", request.id);
            return;
        };
        if !self.speech.audible() {
            log::info!("voice for {}: speech is off", request.id);
            return;
        }
        let speech = match self.speech() {
            Ok(speech) => speech,
            Err(e) => {
                log::warn!("voice for {}: {e}", request.id);
                return;
            }
        };
        let Some(event) = speech.dispatcher().find(EVENT).map(|event| event.id) else {
            log::warn!("voice for {}: the speech database has no {EVENT} event", request.id);
            return;
        };
        let params = take_params(take);
        if request.cut {
            if let Err(e) = speech.play_now(event, 0, &params) {
                log::warn!("voice for {}: {e}", request.id);
            }
            return;
        }
        let outcome = speech.request_with(event, 0, params);
        log::info!("voice for {}: {outcome:?}", request.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::speech::params::lib_params;

    #[test]
    fn requests_are_taken_once_in_order_and_replays_cut() {
        request(10);
        replay(96);
        request(17);
        let taken = take();
        assert_eq!(
            taken,
            [
                VoiceRequest { id: 10, cut: false },
                VoiceRequest { id: 96, cut: true },
                VoiceRequest { id: 17, cut: false },
            ]
        );
        assert!(take().is_empty(), "a request is taken once");
    }

    #[test]
    fn a_call_and_a_replay_of_the_same_id_are_two_requests() {
        request(95);
        replay(95);
        assert_eq!(take().len(), 2);
    }

    #[test]
    fn the_take_of_an_id_follows_the_group_table() {
        assert_eq!(recorded_take(10), Some(VoiceTake { group: 16, line: 0 }));
        assert_eq!(recorded_take(96), Some(VoiceTake { group: 1, line: 7 }));
        assert_eq!(recorded_take(17), Some(VoiceTake { group: 15, line: 0 }));
    }

    #[test]
    fn ids_without_a_recording_have_no_take() {
        assert_eq!(recorded_take(16), None, "the prologue's sixth call has no recording");
        assert_eq!(recorded_take(9), None);
        assert_eq!(recorded_take(106), None);
        assert_eq!(recorded_take(120), None, "a tip has no voice");
    }

    #[test]
    fn the_take_parameters_are_the_one_bit_masks_of_group_and_line() {
        let params = take_params(VoiceTake { group: 16, line: 6 });
        assert_eq!(params.get(Param::CellCallBucket), Some(1 << 16));
        assert_eq!(params.get(Param::CellCallNumber), Some(1 << 6));
        assert_eq!(take_params(VoiceTake { group: 0, line: 0 }).get(Param::CellCallBucket), Some(1));
    }

    #[test]
    fn the_cellcall_event_numbers_its_parameters_after_no_speaker() {
        let params = take_params(VoiceTake { group: 1, line: 7 });
        let numbered = lib_params(EVENT, 0, &params);
        assert_eq!(numbered.get(1), Some(1 << 1), "cell_call_bucket is the first argument");
        assert_eq!(numbered.get(2), Some(1 << 7), "cell_call_number is the second");
    }
}
