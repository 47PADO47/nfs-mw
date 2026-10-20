//! The index of a speech `.big`: which streams are one voice's lines of one phrase, and how phrases make sentences.
//!
//! EA's speech library (SPCH) keeps its recordings in a `.big` ([`crate::big`]) and describes them in a small
//! `.idx` file: a table of *banks* and, after the table, a header per bank. A bank is the set of takes of one
//! phrase by one speaker, stored back to back in the `.big`. [`SpeechIndex`] reads the file; [`SpeechBank`] gives
//! each take's position, and [`SpeechBank::decode`] decodes it. Which phrases make up a sentence, and which takes of
//! them fit a request's parameters, is in the event database (`.evt`), read by [`Sentences`]; [`Assembler`] turns an
//! event and its [`Params`] into the takes to say. Layouts: `docs/formats/audio.md`, `docs/formats/speech-evt.md`.

mod assemble;
mod bank;
mod evt;
mod index;
mod params;
mod sentence;

pub use assemble::{Assembler, Chosen, Options, Random, Repeats};
pub use bank::{BankHeader, SAMPLE_UNIT};
pub use evt::Sentences;
pub use index::{SpeechBank, SpeechIndex};
pub use params::{MAX_PARAMS, Params};
pub use sentence::{Block, Entry, Sentence, Test, Voice};
