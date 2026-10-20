//! Building a sentence from the recordings: which takes say it for a given request.
//!
//! The steps follow the speech library of the original game (spec `docs/specs/speech-sentences.md`):
//!
//! 1. The versions (blocks) of the event's sentence are tried in a random order weighted by [`Block::weight`]; each
//!    is considered with its `chance` percent.
//! 2. In a version, every entry needs a bank (the phrase, in the requested speaker's voice when the entry says so)
//!    and, in that bank, a take that fits: for the nth test, the take's nth selector bit must be in the test's mask and
//!    set in the request's parameter. A take that was said lately (the bank's repeat memory) is left out unless every
//!    fitting take was. If an entry has no take, the version fails and the next one is tried.
//! 3. The takes of a version that works, one per entry, are the sentence. They are remembered as said.

use std::collections::{HashMap, VecDeque};

use crate::speech::Sentences;
use crate::speech::index::SpeechIndex;
use crate::speech::params::Params;
use crate::speech::sentence::{Block, Entry, Voice};

/// The bank speaker number of recordings made for every voice.
const ANY_SPEAKER: u16 = 0xFFFF;

/// A source of random numbers for the choices of a sentence.
pub trait Random {
    /// A number in `0..n` (0 when `n` is 0).
    fn below(&mut self, n: usize) -> usize;
}

/// Switches for the parts of the original that a rewrite may want different.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// When the requested speaker did not record the sentence (or no speaker was requested), let another voice that
    /// did say it. The original drops the sentence instead; the rewrite's callers do not always know who recorded
    /// what, so the default keeps the line.
    pub speaker_fallback: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { speaker_fallback: true }
    }
}

/// One take of the sentence: the bank (its position in [`SpeechIndex::banks`]) and the take in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chosen {
    pub bank: usize,
    pub take: usize,
}

/// The takes said last in each bank, newest first.
#[derive(Debug, Clone, Default)]
pub struct Repeats {
    said: HashMap<usize, VecDeque<usize>>,
}

impl Repeats {
    /// How long ago `take` of `bank` was said: 1 for the last take said, 2 for the one before; 0 when it is not in
    /// the bank's memory.
    pub fn age(&self, bank: usize, take: usize) -> usize {
        let Some(said) = self.said.get(&bank) else { return 0 };
        said.iter().position(|&t| t == take).map_or(0, |at| at + 1)
    }

    /// Remember that `take` of `bank` was said, keeping at most `memory` takes.
    pub fn remember(&mut self, bank: usize, take: usize, memory: usize) {
        if memory == 0 {
            return;
        }
        let said = self.said.entry(bank).or_default();
        said.push_front(take);
        said.truncate(memory);
    }

    /// Forget everything said.
    pub fn clear(&mut self) {
        self.said.clear();
    }
}

/// The sentences and the recordings they are built from.
#[derive(Debug, Clone, Copy)]
pub struct Assembler<'a> {
    pub sentences: &'a Sentences,
    pub index: &'a SpeechIndex,
    pub options: Options,
}

impl<'a> Assembler<'a> {
    pub fn new(sentences: &'a Sentences, index: &'a SpeechIndex) -> Self {
        Self { sentences, index, options: Options::default() }
    }

    /// The takes that say `event` for `params`, in order; `None` when the event has no sentence or no version of it
    /// can be said (a take is missing for these parameters).
    pub fn assemble(
        &self,
        event: u16,
        params: &Params,
        repeats: &mut Repeats,
        rng: &mut dyn Random,
    ) -> Option<Vec<Chosen>> {
        let sentence = self.sentences.sentence(event)?;
        for at in try_order(&sentence.blocks, rng) {
            let block = &sentence.blocks[at];
            if rng.below(100) >= usize::from(block.chance) || block.entries.is_empty() {
                continue;
            }
            let Some(chosen) = self.version(block, params, repeats, rng) else { continue };
            for c in &chosen {
                let memory = usize::from(self.index.banks()[c.bank].header.memory);
                repeats.remember(c.bank, c.take, memory);
            }
            return Some(chosen);
        }
        None
    }

    /// The takes of one version, or `None` when an entry cannot be said.
    fn version(&self, block: &Block, params: &Params, repeats: &Repeats, rng: &mut dyn Random) -> Option<Vec<Chosen>> {
        let voices = self.voices(block, params, rng)?;
        let mut chosen = Vec::with_capacity(block.entries.len());
        for entry in &block.entries {
            let bank = self.bank_of(entry, &voices)?;
            let take = self.take_of(bank, entry, params, repeats, rng)?;
            chosen.push(Chosen { bank, take });
        }
        Some(chosen)
    }

    /// The speaker of every parameter number the block takes a voice from: the requested one when it recorded all the
    /// block's phrases, else (with [`Options::speaker_fallback`]) one that did.
    fn voices(&self, block: &Block, params: &Params, rng: &mut dyn Random) -> Option<Vec<(u8, u16)>> {
        let mut voices: Vec<(u8, u16)> = Vec::new();
        for entry in &block.entries {
            let Voice::Speaker(number) = entry.voice else { continue };
            if voices.iter().any(|&(n, _)| n == number) {
                continue;
            }
            let able = self.speakers_for(block, number);
            let wanted = params.get(number).and_then(|v| u16::try_from(v).ok()).filter(|&s| s != 0);
            match wanted {
                Some(speaker) if able.contains(&speaker) => voices.push((number, speaker)),
                Some(_) if !self.options.speaker_fallback => return None,
                _ if able.is_empty() => return None,
                _ => voices.push((number, able[rng.below(able.len())])),
            }
        }
        Some(voices)
    }

    /// The speakers that recorded every phrase the block takes from parameter `number`, in speaker order.
    fn speakers_for(&self, block: &Block, number: u8) -> Vec<u16> {
        let mut able: Option<Vec<u16>> = None;
        for entry in &block.entries {
            if entry.voice != Voice::Speaker(number) {
                continue;
            }
            let mut here: Vec<u16> =
                self.index.banks_for(entry.phrase).map(|b| b.header.speaker).filter(|&s| s != ANY_SPEAKER).collect();
            here.sort_unstable();
            here.dedup();
            able = Some(match able {
                None => here,
                Some(so_far) => so_far.into_iter().filter(|s| here.contains(s)).collect(),
            });
        }
        able.unwrap_or_default()
    }

    /// The bank an entry reads: for a speaker entry the one of that speaker, else the bank for every voice (or any).
    fn bank_of(&self, entry: &Entry, voices: &[(u8, u16)]) -> Option<usize> {
        let banks = self.index.banks();
        let of_phrase = banks.iter().enumerate().filter(|(_, b)| b.header.event == entry.phrase);
        let Voice::Speaker(number) = entry.voice else {
            let mut all: Vec<(usize, u16)> = of_phrase.map(|(i, b)| (i, b.header.speaker)).collect();
            all.sort_by_key(|&(_, s)| std::cmp::Reverse(s == ANY_SPEAKER));
            return all.first().map(|&(i, _)| i);
        };
        let speaker = voices.iter().find(|&&(n, _)| n == number)?.1;
        let mut found = of_phrase.filter(|(_, b)| b.header.speaker == speaker);
        found.next().map(|(i, _)| i)
    }

    /// A take of `bank` that fits the entry's tests and `params`, not said lately when possible.
    fn take_of(
        &self,
        bank: usize,
        entry: &Entry,
        params: &Params,
        repeats: &Repeats,
        rng: &mut dyn Random,
    ) -> Option<usize> {
        let header = &self.index.banks()[bank].header;
        let fitting: Vec<usize> =
            (0..header.takes()).filter(|&t| fits(header.selectors_of(t), entry, params)).collect();
        let fresh: Vec<usize> = fitting.iter().copied().filter(|&t| repeats.age(bank, t) == 0).collect();
        if !fresh.is_empty() {
            return Some(fresh[rng.below(fresh.len())]);
        }
        // Every fitting take was said lately: the one said longest ago goes again.
        fitting.iter().copied().max_by_key(|&t| (repeats.age(bank, t), std::cmp::Reverse(t)))
    }
}

/// Whether a take with these selector bits fits the entry's tests under `params`.
fn fits(selectors: &[u8], entry: &Entry, params: &Params) -> bool {
    if selectors.len() < entry.tests.len() {
        return false;
    }
    entry.tests.iter().zip(selectors).all(|(test, &bit)| {
        if bit >= 32 || test.mask & (1 << bit) == 0 {
            return false;
        }
        test.param == 0 || params.allows(test.param, bit)
    })
}

/// The order the versions are tried in: a random order weighted by their weights, then the versions of weight 0 in
/// file order.
fn try_order(blocks: &[Block], rng: &mut dyn Random) -> Vec<usize> {
    let mut left: Vec<usize> = (0..blocks.len()).filter(|&i| blocks[i].weight > 0).collect();
    let mut order = Vec::with_capacity(blocks.len());
    while !left.is_empty() {
        let total: usize = left.iter().map(|&i| blocks[i].weight as usize).sum();
        let mut roll = rng.below(total);
        let mut pick = left.len() - 1;
        for (n, &i) in left.iter().enumerate() {
            let weight = blocks[i].weight as usize;
            if roll < weight {
                pick = n;
                break;
            }
            roll -= weight;
        }
        order.push(left.remove(pick));
    }
    order.extend((0..blocks.len()).filter(|&i| blocks[i].weight == 0));
    order
}
