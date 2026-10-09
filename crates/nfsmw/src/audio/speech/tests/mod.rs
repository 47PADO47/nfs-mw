//! Unit tests on synthetic events and recordings. Nothing here comes from the game.

mod dispatcher;
mod lines;
mod real;
mod rules;
mod speech;

use nfsmw_data::speech::SpeechEvent;

use super::Voice;

/// An event with the default scheduling data and the given number, changed by `edit`.
pub fn event(id: u32, edit: impl FnOnce(&mut SpeechEvent)) -> SpeechEvent {
    let mut e = SpeechEvent { id, name: format!("event{id}"), ..SpeechEvent::default() };
    edit(&mut e);
    e
}

/// A voice that records what it is asked to say and sounds until `end` is called.
#[derive(Default)]
pub struct FakeVoice {
    pub busy: bool,
    /// (event, speaker, cut) for every line started.
    pub plays: Vec<(u32, u16, bool)>,
    /// Events that have nothing to say.
    pub mute: Vec<u32>,
}

impl FakeVoice {
    pub fn ids(&self) -> Vec<u32> {
        self.plays.iter().map(|p| p.0).collect()
    }
}

impl Voice for FakeVoice {
    fn busy(&self) -> bool {
        self.busy
    }

    fn play(&mut self, event: &SpeechEvent, speaker: u16, cut: bool) -> bool {
        if self.mute.contains(&event.id) {
            return false;
        }
        self.plays.push((event.id, speaker, cut));
        self.busy = true;
        true
    }
}
