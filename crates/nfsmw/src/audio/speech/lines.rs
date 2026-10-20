//! The recordings: `copspeech.big` and its index, read with positioned reads (the file is 190 MB).
//!
//! A line is an event of the speech database. Almost every event is a *sentence*: `copspeech.evt` lists the phrases
//! it says in order, and which take of each phrase fits the request's parameters (the car's colour, the direction,
//! the unit calling); `ea_audio::speech::Assembler` chooses them and they are joined into one sound. An event with
//! recordings but no sentence is said as it is: a bank of the requested speaker and a take that was not said lately.
//! Spec `docs/specs/speech-sentences.md`.

use std::fs::File;
use std::path::Path;

use ea_audio::speech::{Assembler, Chosen, Params, Repeats, Sentences, SpeechBank, SpeechIndex};
use ea_audio::{Pcm, ReadAt};

use super::random::Rng;

/// The speaker number of recordings that any unit may say.
const ANY_SPEAKER: u16 = 0xFFFF;

/// Where lines come from.
pub trait Lines {
    /// Whether `event` has recordings of its own (a bank with its number).
    fn has(&self, event: u32) -> bool;

    /// Whether `event` can be said at all: it has a sentence, or recordings of its own.
    fn says(&self, event: u32) -> bool {
        self.has(event)
    }

    /// The line of `event` said by `speaker` (0 for any speaker, or when that speaker did not record it), for the
    /// situation `params` describes (numbered as the speech database tests them; see `params::lib_params`).
    fn take(&mut self, event: u32, speaker: u16, params: &Params) -> Result<Pcm, String>;

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

/// The install's `copspeech.idx`, `copspeech.evt` and `copspeech.big`.
pub struct Files {
    index: SpeechIndex,
    sentences: Sentences,
    big: FileSource,
    /// The takes said last, per bank.
    repeats: Repeats,
    rng: Rng,
}

impl Files {
    pub fn open(index: &[u8], evt: &[u8], big: &Path, rng: Rng) -> Result<Self, String> {
        let index = SpeechIndex::parse(index).map_err(|e| format!("copspeech.idx: {e}"))?;
        let sentences = Sentences::parse(evt).map_err(|e| format!("copspeech.evt: {e}"))?;
        let big = FileSource::open(big).map_err(|e| format!("{}: {e}", big.display()))?;
        Ok(Self { index, sentences, big, repeats: Repeats::default(), rng })
    }

    /// The takes that say `event`: its sentence under `params`, or (no sentence) one take of a bank of its own.
    fn parts(&mut self, event: u16, speaker: u16, params: &Params) -> Result<Vec<Chosen>, String> {
        if self.sentences.sentence(event).is_some() {
            let assembler = Assembler::new(&self.sentences, &self.index);
            let parts = assembler.assemble(event, params, &mut self.repeats, &mut self.rng);
            return parts.ok_or_else(|| format!("event {event}: no version of the sentence fits its parameters"));
        }
        let banks: Vec<&SpeechBank> = self.index.banks_for(event).collect();
        let Some(bank) = choose_bank(&banks, speaker, &mut self.rng) else {
            return Err(format!("event {event} has no sentence and no recording"));
        };
        let at = bank.number as usize;
        let take = choose_take(&bank.header, at, &self.repeats, &mut self.rng);
        self.repeats.remember(at, take, usize::from(bank.header.memory));
        Ok(vec![Chosen { bank: at, take }])
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

/// Choose a take of a bank that was not said lately (the bank's own repeat memory decides what is lately); when
/// every take was, the one said longest ago.
pub fn choose_take(header: &ea_audio::speech::BankHeader, bank: usize, repeats: &Repeats, rng: &mut Rng) -> usize {
    let fresh: Vec<usize> = (0..header.takes()).filter(|&t| repeats.age(bank, t) == 0).collect();
    if let Some(&take) = fresh.get(rng.below(fresh.len())) {
        return take;
    }
    (0..header.takes()).max_by_key(|&t| (repeats.age(bank, t), std::cmp::Reverse(t))).unwrap_or(0)
}

impl Lines for Files {
    fn has(&self, event: u32) -> bool {
        u16::try_from(event).is_ok_and(|e| self.index.banks_for(e).next().is_some())
    }

    fn says(&self, event: u32) -> bool {
        self.has(event) || u16::try_from(event).is_ok_and(|e| self.sentences.sentence(e).is_some())
    }

    fn take(&mut self, event: u32, speaker: u16, params: &Params) -> Result<Pcm, String> {
        let wanted = u16::try_from(event).map_err(|_| format!("event {event} is not a speech event"))?;
        let mut pcms = Vec::new();
        for part in self.parts(wanted, speaker, params)? {
            let bank = &self.index.banks()[part.bank];
            let pcm = bank
                .decode(&self.big, part.take)
                .map_err(|e| format!("bank {} take {}: {e}", bank.number, part.take))?;
            pcms.push(pcm);
        }
        join(pcms)
    }

    fn bank_take(&self, bank: usize, take: usize) -> Result<Pcm, String> {
        let banks = self.index.banks();
        let bank = banks.get(bank).ok_or_else(|| format!("there are {} banks", banks.len()))?;
        bank.decode(&self.big, take).map_err(|e| format!("bank {} take {take}: {e}", bank.number))
    }

    fn summary(&self) -> String {
        format!(
            "{} banks, {} takes, {} sentences",
            self.index.banks().len(),
            self.index.take_count(),
            self.sentences.len()
        )
    }
}
