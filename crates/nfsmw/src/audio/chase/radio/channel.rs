//! The radio queue as the machine sees it: the dispatcher's busy state, history and queue, with the machine's lines
//! mapped to the events of the attribute database.

use std::collections::HashMap;

use blackbox_pursuit::speech::{Channel, Line, Params, Speaker};

use super::params::event_name;
use crate::audio::speech::Dispatcher;

/// The event number of each line, and the line of each event number.
#[derive(Debug, Default)]
pub struct LineIds {
    by_line: Vec<Option<u32>>,
    by_event: HashMap<u32, Line>,
}

impl LineIds {
    /// Looks every line up among the dispatcher's events by name.
    pub fn new(dispatcher: &Dispatcher) -> Self {
        let mut ids = Self { by_line: vec![None; Line::ALL.len()], by_event: HashMap::new() };
        for &line in Line::ALL {
            let Some(event) = dispatcher.find(event_name(line)) else { continue };
            ids.by_line[line as usize] = Some(event.id);
            // Two collections can share a number (the world collision and the spin): the first line keeps it.
            ids.by_event.entry(event.id).or_insert(line);
        }
        ids
    }

    pub fn id(&self, line: Line) -> Option<u32> {
        self.by_line.get(line as usize).copied().flatten()
    }

    pub fn line(&self, event: u32) -> Option<Line> {
        self.by_event.get(&event).copied()
    }

    /// The lines that have no event in the database.
    pub fn missing(&self) -> Vec<Line> {
        Line::ALL.iter().copied().filter(|l| self.id(*l).is_none()).collect()
    }
}

/// What the machine asked for per event: the line, the voice and the typed parameters, kept until the line is said.
pub type Sent = HashMap<u32, (Line, Speaker, Params)>;

/// The dispatcher as a [`Channel`].
pub struct DispatcherChannel<'a> {
    pub dispatcher: &'a Dispatcher,
    pub ids: &'a LineIds,
    pub sent: &'a Sent,
    pub last: Option<(Line, Speaker)>,
}

impl Channel for DispatcherChannel<'_> {
    fn busy(&self) -> bool {
        self.dispatcher.busy()
    }

    fn dead_air(&self) -> f32 {
        self.dispatcher.silent_secs()
    }

    fn said(&self, line: Line) -> u32 {
        self.ids.id(line).map_or(0, |id| self.dispatcher.history().count(id))
    }

    fn queued(&self, line: Line) -> bool {
        self.ids.id(line).is_some_and(|id| self.dispatcher.is_queued(id))
    }

    fn playing(&self) -> Option<(Line, Params)> {
        let (event, _) = self.dispatcher.playing()?;
        let (line, _, params) = self.sent.get(&event)?;
        Some((*line, *params))
    }

    fn last(&self) -> Option<(Line, Speaker)> {
        self.last
    }
}

/// A radio with nothing on it: the speech is not loaded yet.
pub struct Silent;

impl Channel for Silent {
    fn busy(&self) -> bool {
        false
    }

    fn dead_air(&self) -> f32 {
        65535.0
    }

    fn said(&self, _line: Line) -> u32 {
        0
    }

    fn queued(&self, _line: Line) -> bool {
        false
    }

    fn playing(&self) -> Option<(Line, Params)> {
        None
    }

    fn last(&self) -> Option<(Line, Speaker)> {
        None
    }
}
