//! A track as a list of streams: the graph walk joined with the stream table.

use super::Mpf;
use super::graph::{Graph, NodeKind, WalkEnd};
use crate::error::{Error, Result};

/// One stream of a chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// The audio node that plays the stream.
    pub node: usize,
    /// Index into [`Mpf::streams`].
    pub stream: usize,
    /// Where the segment starts in the chain, in milliseconds of stored durations.
    pub start_ms: u64,
    /// Stored duration of the stream in milliseconds.
    pub duration_ms: u32,
}

/// The streams a track plays one after another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    pub segments: Vec<Segment>,
    pub end: WalkEnd,
}

impl Chain {
    /// Total length in milliseconds (sum of the stored durations).
    pub fn total_ms(&self) -> u64 {
        self.segments.last().map_or(0, |s| s.start_ms + u64::from(s.duration_ms))
    }

    /// Total length in seconds.
    pub fn total_secs(&self) -> f64 {
        self.total_ms() as f64 / 1000.0
    }
}

impl Mpf {
    /// The chain of streams of the track that starts at node `start` (see [`Graph::walk`]).
    pub fn chain(&self, graph: &Graph, start: usize, value: u8) -> Result<Chain> {
        let walk = graph.walk(start, value)?;
        let mut segments = Vec::new();
        let mut start_ms = 0u64;
        for node in walk.audio(graph) {
            let NodeKind::Audio { stream } = graph.nodes[node].kind else { continue };
            let stream = stream as usize;
            let entry = self.streams.get(stream).ok_or(Error::NoSuchEntry(stream))?;
            segments.push(Segment { node, stream, start_ms, duration_ms: entry.duration_ms });
            start_ms += u64::from(entry.duration_ms);
        }
        Ok(Chain { segments, end: walk.end })
    }

    /// The chain of the song whose event has id `event_id` (low 24 bits compared), at control value `value`.
    pub fn song_chain(&self, graph: &Graph, event_id: u32, value: u8) -> Result<Chain> {
        let start = graph.song_start(event_id).ok_or(Error::Corrupt("no event starts a song with this id"))?;
        self.chain(graph, start, value)
    }
}
