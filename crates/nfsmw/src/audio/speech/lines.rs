//! The recordings: `copspeech.big` and its index, read with positioned reads (the file is 190 MB).
//!
//! A phrase of the speech database (an event number) is recorded by several speakers, several takes each. Only
//! phrases that have banks of their own can be said as they are; the events that are sentences built from several
//! phrases need the rules of `copspeech.evt`, which are not read yet (spec `docs/specs/speech.md` §5).

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::path::Path;

use ea_audio::speech::{SpeechBank, SpeechIndex};
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
    big: FileSource,
    /// The takes said last, per bank, newest first.
    recent: HashMap<u32, VecDeque<usize>>,
    rng: Rng,
}

impl Files {
    pub fn open(index: &[u8], big: &Path, rng: Rng) -> Result<Self, String> {
        let index = SpeechIndex::parse(index).map_err(|e| format!("copspeech.idx: {e}"))?;
        let big = FileSource::open(big).map_err(|e| format!("{}: {e}", big.display()))?;
        Ok(Self { index, big, recent: HashMap::new(), rng })
    }
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
        let wanted = u16::try_from(event).map_err(|_| format!("event {event} is not a phrase"))?;
        let banks: Vec<&SpeechBank> = self.index.banks_for(wanted).collect();
        let none = || format!("event {event} is a sentence built from several phrases: its rules are not read yet");
        let bank = choose_bank(&banks, speaker, &mut self.rng).ok_or_else(none)?;
        let recent = self.recent.entry(bank.number).or_default();
        let take = choose_take(bank.header.takes(), recent, &mut self.rng);
        bank.decode(&self.big, take).map_err(|e| format!("event {event}: {e}"))
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
