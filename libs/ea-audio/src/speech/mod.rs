//! The index of a speech `.big`: which streams are one voice's lines of one phrase.
//!
//! EA's speech library (SPCH) keeps its recordings in a `.big` ([`crate::big`]) and describes them in a small
//! `.idx` file: a table of *banks* and, after the table, a header per bank. A bank is the set of takes of one
//! phrase by one speaker, stored back to back in the `.big`. [`SpeechIndex`] reads the file; [`SpeechBank`] gives
//! each take's position, and [`SpeechBank::decode`] decodes it. Which phrases make up a sentence is the business of
//! the event database (`.evt`), which this module does not read. Layout: `docs/formats/audio.md`.

mod bank;
mod index;

pub use bank::{BankHeader, SAMPLE_UNIT};
pub use index::{SpeechBank, SpeechIndex};
