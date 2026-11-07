//! The chase radio's line events, for a trace recorder: each line the speech machine asked the speech for (with the
//! queue's answer) and each line the speech finished saying, with its event name and number and the voice. The radio
//! keeps the last [`CAPACITY`] until a reader takes them ([`super::ChaseRadio::take_line_events`]); nothing reads them
//! unless a recorder does, and the list never grows past its capacity. A pursuit trace, which runs in the world and
//! not beside the audio, opens its own copy with [`open_recording`] and drains it with [`take_recorded`].

use std::collections::VecDeque;
use std::sync::Mutex;

use blackbox_pursuit::speech::{Line, Params, Speaker};

use super::params::event_name;
use crate::audio::speech::{Dropped, Outcome};

/// The events kept until a reader takes them: the oldest go first.
pub const CAPACITY: usize = 64;

/// The events a pursuit trace keeps between two of its samples at most; the oldest go first.
const RECORDING_CAPACITY: usize = 1024;

/// The pursuit trace's copy of the events: `None` while no trace records.
static RECORDING: Mutex<Option<VecDeque<RadioLineEvent>>> = Mutex::new(None);

/// Starts (`true`) or stops keeping a copy of every line event for a pursuit trace.
pub fn open_recording(on: bool) {
    if let Ok(mut recording) = RECORDING.lock() {
        *recording = on.then(VecDeque::new);
    }
}

/// The line events since the last call, oldest first; none while no trace records.
pub fn take_recorded() -> Vec<RadioLineEvent> {
    let Ok(mut recording) = RECORDING.lock() else { return Vec::new() };
    recording.as_mut().map(|events| events.drain(..).collect()).unwrap_or_default()
}

fn record(event: RadioLineEvent) {
    let Ok(mut recording) = RECORDING.lock() else { return };
    let Some(events) = recording.as_mut() else { return };
    if events.len() == RECORDING_CAPACITY {
        events.pop_front();
    }
    events.push_back(event);
}

/// What happened to a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStage {
    /// The machine asked for it; the speech queue answered `Outcome`.
    Asked(Outcome),
    /// The speech finished saying it.
    Said,
    /// It left the speech queue without being said, and why.
    Dropped(Dropped),
}

/// One line of the chase radio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadioLineEvent {
    pub stage: LineStage,
    /// The machine's line.
    pub line: Line,
    /// The speech event's name in the attribute database (`event_name`), and its number.
    pub event: &'static str,
    pub event_id: u32,
    /// The voice: the speaker number the machine picked (0 for any).
    pub speaker: Speaker,
    /// The line's typed parameters (the wording).
    pub params: Params,
}

/// The kept events.
#[derive(Debug, Default)]
pub struct LineLog {
    events: VecDeque<RadioLineEvent>,
}

impl LineLog {
    /// Keeps an event, dropping the oldest at capacity.
    pub fn push(&mut self, stage: LineStage, line: Line, event_id: u32, speaker: Speaker, params: Params) {
        if self.events.len() == CAPACITY {
            self.events.pop_front();
        }
        let event = event_name(line);
        let entry = RadioLineEvent { stage, line, event, event_id, speaker, params };
        self.events.push_back(entry);
        record(entry);
        log::trace!(target: "chase_radio_lines", "{stage:?} {event} (event {event_id}) voice {speaker}");
    }

    /// The events kept since the last call, oldest first.
    pub fn take(&mut self) -> Vec<RadioLineEvent> {
        self.events.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recording_copies_the_events_until_it_is_closed() {
        // Other tests push lines at the same time, so this one looks for its own event number only.
        let line = Line::ALL[0];
        open_recording(true);
        LineLog::default().push(LineStage::Said, line, 0xC0FFEE, 3, Params::default());
        let taken = take_recorded();
        open_recording(false);
        assert!(taken.iter().any(|e| e.event_id == 0xC0FFEE && e.speaker == 3), "the open recording copies the event");
        LineLog::default().push(LineStage::Said, line, 0xC0FFEF, 3, Params::default());
        assert!(take_recorded().is_empty(), "a closed recording keeps nothing");
    }

    #[test]
    fn the_log_keeps_the_latest_events_in_order_and_empties_when_taken() {
        let mut log = LineLog::default();
        let line = Line::ALL[0];
        for id in 0..(CAPACITY as u32 + 3) {
            log.push(LineStage::Said, line, id, 2, Params::default());
        }
        let taken = log.take();
        assert_eq!(taken.len(), CAPACITY, "the oldest go at capacity");
        assert_eq!(taken.first().map(|e| e.event_id), Some(3));
        assert_eq!(
            taken.last().map(|e| (e.event_id, e.speaker, e.event)),
            Some((CAPACITY as u32 + 2, 2, event_name(line)))
        );
        assert!(log.take().is_empty());
    }
}
