//! The chase radio's line events, for a trace recorder: each line the speech machine asked the speech for (with the
//! queue's answer) and each line the speech finished saying, with its event name and number and the voice. The radio
//! keeps the last [`CAPACITY`] until a reader takes them ([`super::ChaseRadio::take_line_events`]); nothing reads them
//! unless a recorder does, and the list never grows past its capacity.

use std::collections::VecDeque;

use blackbox_pursuit::speech::{Line, Params, Speaker};

use super::params::event_name;
use crate::audio::speech::Outcome;

/// The events kept until a reader takes them: the oldest go first.
pub const CAPACITY: usize = 64;

/// What happened to a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStage {
    /// The machine asked for it; the speech queue answered `Outcome`.
    Asked(Outcome),
    /// The speech finished saying it.
    Said,
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
        self.events.push_back(RadioLineEvent { stage, line, event, event_id, speaker, params });
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
