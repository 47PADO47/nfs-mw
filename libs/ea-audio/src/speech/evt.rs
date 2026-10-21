//! The event database `copspeech.evt`: which phrases make up each sentence, and how parameters choose among them.
//!
//! The file is a header, a table of record positions and the records ([`Sentence`]). Layout:
//! `docs/formats/speech-evt.md`. Assembling a sentence from the recordings is in [`crate::speech::assemble`].

use std::collections::HashMap;

use crate::bytes::{slice, u16_le, u32_le};
use crate::error::Result;
use crate::speech::sentence::Sentence;

/// Where the record count sits, then the record positions (each `u16`, in units of 4 bytes) from `TABLE_AT`.
const COUNT_AT: usize = 0x10;
const TABLE_AT: usize = 0x18;

/// The sentences of a `copspeech.evt`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sentences {
    sentences: HashMap<u16, Sentence>,
}

impl Sentences {
    /// Parse a `copspeech.evt`.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let count = u32_le(data, COUNT_AT)? as usize;
        let mut starts = Vec::with_capacity(count);
        for i in 0..count {
            starts.push(usize::from(u16_le(data, TABLE_AT + 2 * i)?) * 4);
        }
        let mut sentences = HashMap::new();
        for &start in &starts {
            let end = starts.iter().filter(|&&s| s > start).min().copied().unwrap_or(data.len());
            let sentence = Sentence::parse(slice(data, start, end.saturating_sub(start))?)?;
            sentences.insert(sentence.event, sentence);
        }
        Ok(Self { sentences })
    }

    /// The sentence of event `event`, if the database has one.
    pub fn sentence(&self, event: u16) -> Option<&Sentence> {
        self.sentences.get(&event)
    }

    /// How many sentences the database holds.
    pub fn len(&self) -> usize {
        self.sentences.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sentences.is_empty()
    }

    /// The events that have a sentence, in no particular order.
    pub fn events(&self) -> impl Iterator<Item = u16> + '_ {
        self.sentences.keys().copied()
    }
}
