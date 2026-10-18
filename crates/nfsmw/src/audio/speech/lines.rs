//! The recordings: `copspeech.big` and its index, read with positioned reads (the file is 190 MB).
//!
//! A phrase of the speech database (an event number) is recorded by several speakers, several takes each. A phrase
//! with banks of its own is said as it is. An event with no banks is a sentence: `copspeech.evt` names the phrases it
//! says, and they are joined in file order (the parameter tests that choose variants are not read yet; spec
//! `docs/specs/speech.md` §5).

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::path::Path;

use ea_audio::speech::{Sentences, SpeechBank, SpeechIndex};
use ea_audio::{Pcm, ReadAt};

use super::random::Rng;

/// The speaker number of recordings that any unit may say.
const ANY_SPEAKER: u16 = 0xFFFF;

/// Where lines come from.
pub trait Lines {
    /// Whether `event` has recordings of its own.
    fn has(&self, event: u32) -> bool;

    /// A take of `event` said by `speaker` (0 for any speaker, or when that speaker did not record it).
    fn take(&mut self, event: u32, speaker: u16) -> Result<Pcm, String>;

    /// Take `take` of bank `bank` (numbers of the index), for listening to recordings one by one.
    fn bank_take(&self, bank: usize, take: usize) -> Result<Pcm, String>;

    /// What the recordings hold, in a line.
    fn summary(&self) -> String;
}

/// A file read with positioned reads, so it is never loaded whole.
pub struct FileSource {
    file: File,
    len: u64,
}

impl FileSource {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }
}

impl ReadAt for FileSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> usize {
        let mut done = 0;
        while done < buf.len() {
            #[cfg(windows)]
            let n = std::os::windows::fs::FileExt::seek_read(&self.file, &mut buf[done..], offset + done as u64);
            #[cfg(unix)]
            let n = std::os::unix::fs::FileExt::read_at(&self.file, &mut buf[done..], offset + done as u64);
            match n {
                Ok(0) | Err(_) => break,
                Ok(n) => done += n,
            }
        }
        done
    }
}

/// The install's `copspeech.idx` and `copspeech.big`.
pub struct Files {
    index: SpeechIndex,
    sentences: Sentences,
    big: FileSource,
    /// The takes said last, per bank, newest first.
    recent: HashMap<u32, VecDeque<usize>>,
    rng: Rng,
}

/// How deep a sentence may name sentences (a phrase of a phrase): the database's own nesting is shallow.
const MAX_NESTING: usize = 4;

impl Files {
    pub fn open(index: &[u8], evt: &[u8], big: &Path, rng: Rng) -> Result<Self, String> {
        let index = SpeechIndex::parse(index).map_err(|e| format!("copspeech.idx: {e}"))?;
        let sentences = Sentences::parse(evt).map_err(|e| format!("copspeech.evt: {e}"))?;
        let big = FileSource::open(big).map_err(|e| format!("{}: {e}", big.display()))?;
        Ok(Self { index, sentences, big, recent: HashMap::new(), rng })
    }

    /// Takes the phrases of `event` into `parts`: a phrase with banks gives one take, a sentence gives the takes of
    /// its phrases. Phrases with no recording are skipped. Returns how many takes were added.
    fn gather(&mut self, event: u32, speaker: u16, depth: usize, parts: &mut Vec<Pcm>) -> usize {
        let Ok(wanted) = u16::try_from(event) else { return 0 };
        let banks: Vec<&SpeechBank> = self.index.banks_for(wanted).collect();
        if !banks.is_empty() {
            let Some(bank) = choose_bank(&banks, speaker, &mut self.rng) else { return 0 };
            let recent = self.recent.entry(bank.number).or_default();
            let take = choose_take(bank.header.takes(), recent, &mut self.rng);
            let Ok(pcm) = bank.decode(&self.big, take) else { return 0 };
            parts.push(pcm);
            return 1;
        }
        if depth >= MAX_NESTING {
            return 0;
        }
        let phrases = self.sentences.phrases(wanted).to_vec();
        phrases.into_iter().map(|phrase| self.gather(phrase as u32, speaker, depth + 1, parts)).sum()
    }
}

/// The takes of a sentence, one after another. They must share a sample rate and channel count.
fn join(parts: Vec<Pcm>) -> Result<Pcm, String> {
    let mut parts = parts.into_iter();
    let mut out = parts.next().ok_or("no take")?;
    for part in parts {
        if part.sample_rate != out.sample_rate || part.channels != out.channels {
            return Err("the phrases of a sentence differ in format".into());
        }
        out.samples.extend(part.samples);
    }
    out.loop_range = None;
    Ok(out)
}

/// Choose among the banks of a phrase: the ones the speaker recorded, else any.
pub fn choose_bank<'a>(banks: &[&'a SpeechBank], speaker: u16, rng: &mut Rng) -> Option<&'a SpeechBank> {
    let theirs: Vec<&&SpeechBank> = banks
        .iter()
        .filter(|b| speaker != 0 && (b.header.speaker == speaker || b.header.speaker == ANY_SPEAKER))
        .collect();
    if !theirs.is_empty() {
        return Some(theirs[rng.below(theirs.len())]);
    }
    banks.get(rng.below(banks.len())).copied()
}

/// Choose a take of a bank of `count` takes that was not said lately: the ones said last are left out, as many as
/// half the bank (the original's repeat rule is not known; this keeps a speaker from saying one line twice running).
pub fn choose_take(count: usize, recent: &mut VecDeque<usize>, rng: &mut Rng) -> usize {
    let memory = count / 2;
    recent.truncate(memory);
    let fresh: Vec<usize> = (0..count).filter(|t| !recent.contains(t)).collect();
    let take = fresh.get(rng.below(fresh.len())).copied().unwrap_or(0);
    recent.push_front(take);
    recent.truncate(memory);
    take
}

impl Lines for Files {
    fn has(&self, event: u32) -> bool {
        u16::try_from(event).is_ok_and(|e| self.index.banks_for(e).next().is_some())
    }

    fn take(&mut self, event: u32, speaker: u16) -> Result<Pcm, String> {
        let mut parts = Vec::new();
        if self.gather(event, speaker, 0, &mut parts) == 0 {
            return Err(format!("event {event} has no recording, and no phrase of it does"));
        }
        join(parts)
    }

    fn bank_take(&self, bank: usize, take: usize) -> Result<Pcm, String> {
        let banks = self.index.banks();
        let bank = banks.get(bank).ok_or_else(|| format!("there are {} banks", banks.len()))?;
        bank.decode(&self.big, take).map_err(|e| format!("bank {} take {take}: {e}", bank.number))
    }

    fn summary(&self) -> String {
        format!("{} banks, {} takes", self.index.banks().len(), self.index.take_count())
    }
}
