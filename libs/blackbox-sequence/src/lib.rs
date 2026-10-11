//! CARP event systems: bytes and stimuli in, ordered events out.
//! Spec: `docs/formats/event-sequences.md`, `docs/specs/world-sequencers.md`.

use std::collections::BTreeMap;

mod parse;
mod runtime;

pub use runtime::{Emission, Player, QueueMode, RuntimeError, Tags};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Carp(#[from] blackbox_carp::Error),
    #[error("event sequence: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A CARP group; tags refer only to records in this package.
#[derive(Debug, Clone, Default)]
pub struct Package {
    pub engines: Vec<Engine>,
    pub actions: BTreeMap<u32, Action>,
    pub event_lists: BTreeMap<u32, Vec<Event>>,
    pub strings: BTreeMap<u32, String>,
}

#[derive(Debug, Clone)]
pub struct Engine {
    pub name: String,
    pub name_hash: u32,
    pub systems: BTreeMap<u32, System>,
}

#[derive(Debug, Clone)]
pub struct System {
    pub initial: u32,
    pub default: u32,
    pub states: BTreeMap<u32, State>,
}

#[derive(Debug, Clone)]
pub struct State {
    /// Nonzero filters require query/expression evaluation by the host.
    pub filter: Option<u32>,
    pub responses: BTreeMap<u32, Response>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Response {
    pub end_state: u32,
    pub action: u32,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub name_hash: u32,
    /// Seconds; a negative duration never ends automatically.
    pub duration: f32,
    /// Event-tag hash to event-list record tag.
    pub tags: BTreeMap<u32, u32>,
    /// Time in seconds and event-list record tag, in chronological order.
    pub times: Vec<(f32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: u32,
    /// Includes the repeated event ID and any declared alignment padding.
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod runtime_tests;
