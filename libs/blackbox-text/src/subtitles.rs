//! Movie subtitle cues and the lines they show. Spec: `docs/specs/subtitles.md` (records and clock in sections 2
//! and 3, label hashes in 5, tutorial lines in 4.3) and `docs/formats/text.md#subtitles`.
//!
//! A subtitle file is a bare array of 8-byte records with no header: a start time, a padding field and a label hash.
//! The game plays the records in file order, so [`parse`] keeps that order.

use blackbox_hash::{bstring_hash_bytes, bstring_hash_continue};

const RECORD_LEN: usize = 8;
/// The start time that ends a file.
const TERMINATOR: u16 = 0xFFFF;

/// One record of a subtitle file: when a line starts, and the label of its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cue {
    /// Tenths of a second since the movie's first frame.
    pub start_tenths: u16,
    /// The label's hash. The file's clear value means "no text" (see [`SubtitleTrack::new`]).
    pub label: u32,
}

impl Cue {
    /// The start time in seconds.
    pub fn start_secs(&self) -> f32 {
        tenths_to_secs(self.start_tenths)
    }
}

fn tenths_to_secs(tenths: u16) -> f32 {
    f32::from(tenths) / 10.0
}

/// The cues of a subtitle file, in file order.
///
/// Reading stops at the `0xFFFF` terminator, which is not a cue. A file without a terminator ends at its last whole
/// record, and a trailing partial record is ignored. The padding field is not read. Nothing in the bytes is an error,
/// so the result is empty for an empty or truncated file.
pub fn parse(bytes: &[u8]) -> Vec<Cue> {
    let (records, _partial) = bytes.as_chunks::<RECORD_LEN>();
    let mut cues = Vec::new();
    for record in records {
        let start_tenths = u16::from_le_bytes([record[0], record[1]]);
        if start_tenths == TERMINATOR {
            break;
        }
        let label = u32::from_le_bytes([record[4], record[5], record[6], record[7]]);
        cues.push(Cue { start_tenths, label });
    }
    cues
}

/// The hash that keys a subtitle label: the engine's name hash of the label's upper-case form. Spec section 5.1.
pub fn upper_label_hash(label: &str) -> u32 {
    let upper: Vec<u8> = label.bytes().map(|b| b.to_ascii_uppercase()).collect();
    bstring_hash_bytes(&upper)
}

/// The key of a tutorial line: the name hash of `base` continued with `suffix` (`"_A"` or `"_B"`), as if the suffix
/// were appended to the base label. Spec sections 5.1 and 5.2.
pub fn tutorial_line_hash(base: u32, suffix: &str) -> u32 {
    bstring_hash_continue(base, suffix.as_bytes())
}

/// The text of one cue: line 1, and line 2 (tutorials only). Both are `None` for a clear cue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Lines {
    first: Option<String>,
    second: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    start_tenths: u16,
    lines: Lines,
}

/// The lines a movie's subtitles show at each time, built once from its cues.
#[derive(Clone, Debug, Default)]
pub struct SubtitleTrack {
    entries: Vec<Entry>,
}

impl SubtitleTrack {
    /// A track for an ordinary movie: each cue shows its label's text on line 1.
    ///
    /// `clear` is the file's clear value, which the caller passes in. A cue with that label shows nothing, and so does a
    /// label that `resolve` cannot find or that resolves to an empty string.
    pub fn new(cues: &[Cue], clear: u32, resolve: impl Fn(u32) -> Option<String>) -> Self {
        Self::build(cues, clear, |label| Lines { first: text(resolve(label)), second: None })
    }

    /// A track for a tutorial movie, where a cue's label may be a base label that is not in the string table.
    ///
    /// A label that resolves directly shows as in [`SubtitleTrack::new`]. Otherwise line 1 is the base label with
    /// `"_A"` and line 2 is the base with `"_B"`, each looked up with `resolve`. A line that is not found stays hidden.
    pub fn with_tutorial_lines(cues: &[Cue], clear: u32, resolve: impl Fn(u32) -> Option<String>) -> Self {
        Self::build(cues, clear, |label| tutorial_lines(label, &resolve))
    }

    fn build(cues: &[Cue], clear: u32, lines: impl Fn(u32) -> Lines) -> Self {
        let entries = cues
            .iter()
            .map(|cue| Entry { start_tenths: cue.start_tenths, lines: cue_lines(cue.label, clear, &lines) })
            .collect();
        Self { entries }
    }

    /// The line 1 text at `secs`, seconds since the movie's first frame. See [`SubtitleTrack::lines_at`].
    pub fn line_at(&self, secs: f32) -> Option<&str> {
        self.lines_at(secs).0
    }

    /// The text of both lines at `secs`.
    ///
    /// The shown cue is the last one whose start is at or before `secs`. Both lines are `None` before the first cue,
    /// and after a clear cue. Cues are expected in time order, as the files store them.
    pub fn lines_at(&self, secs: f32) -> (Option<&str>, Option<&str>) {
        let reached = self.entries.partition_point(|e| tenths_to_secs(e.start_tenths) <= secs);
        let Some(i) = reached.checked_sub(1) else {
            return (None, None);
        };
        let lines = &self.entries[i].lines;
        (lines.first.as_deref(), lines.second.as_deref())
    }
}

fn cue_lines(label: u32, clear: u32, lines: impl Fn(u32) -> Lines) -> Lines {
    if label == clear {
        return Lines::default();
    }
    lines(label)
}

fn tutorial_lines(label: u32, resolve: &impl Fn(u32) -> Option<String>) -> Lines {
    let direct = text(resolve(label));
    if direct.is_some() {
        return Lines { first: direct, second: None };
    }
    Lines {
        first: text(resolve(tutorial_line_hash(label, "_A"))),
        second: text(resolve(tutorial_line_hash(label, "_B"))),
    }
}

/// A found string, or `None` when it is missing or empty (an empty line shows nothing).
fn text(found: Option<String>) -> Option<String> {
    found.filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests;
