//! Following a track one audio node at a time while the control value changes.
//!
//! [`Graph::walk`](super::Graph::walk) predicts a track for a constant control value. The interactive music
//! cannot: the game writes the control value (0 to 127) while the music plays, and the next node is chosen from
//! it each time a node ends (spec `docs/specs/music-graph.md` section 3). A [`Cursor`] holds the position and
//! takes the value at every step; unlike a walk it may loop for ever, which a pursuit set does.

use super::{Graph, NodeKind, WalkEnd};
use crate::error::{Error, Result};

/// Control nodes passed in a row before the cursor gives up, so a cycle of silent nodes ends the track instead
/// of hanging the caller.
const MAX_CONTROL_RUN: usize = 64;

/// What [`Cursor::advance`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Advance {
    /// An audio node was reached: play stream `stream` (index into the stream table). `fired` lists the 24-bit
    /// ids of the fire-event nodes passed on the way; the cursor does not run them.
    Audio { node: usize, stream: u32, fired: Vec<u32> },
    /// The track is over.
    End(WalkEnd),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum At {
    /// Before the first step: this node has not been entered yet.
    Start(usize),
    /// This audio node has been handed out; the next step leaves it.
    Played(usize),
    Ended(WalkEnd),
}

/// A position in the graph that moves one audio node per [`advance`](Self::advance).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    at: At,
}

impl Cursor {
    /// A cursor that enters the graph at node `start` (a song's or a pursuit set's first node).
    pub fn new(graph: &Graph, start: usize) -> Result<Self> {
        if start >= graph.nodes.len() {
            return Err(Error::NoSuchEntry(start));
        }
        Ok(Self { at: At::Start(start) })
    }

    /// The audio node last handed out, if the cursor is on one.
    pub fn node(&self) -> Option<usize> {
        match self.at {
            At::Played(node) => Some(node),
            _ => None,
        }
    }

    /// Whether the track has ended.
    pub fn ended(&self) -> bool {
        matches!(self.at, At::Ended(_))
    }

    /// Move to the next audio node, choosing every transition with control value `value`. Group heads, end nodes
    /// that lead on and fire-event nodes are passed (fire-event ids are reported, not run). Once the track has
    /// ended every call returns the same end.
    pub fn advance(&mut self, graph: &Graph, value: u8) -> Advance {
        let mut node = match self.at {
            At::Ended(end) => return Advance::End(end),
            At::Start(node) => node,
            At::Played(from) => match graph.next(from, value) {
                Some(next) => next,
                None => return self.end(WalkEnd::DeadEnd),
            },
        };
        let mut fired = Vec::new();
        for _ in 0..=MAX_CONTROL_RUN {
            let current = &graph.nodes[node];
            match current.kind {
                NodeKind::Audio { stream } => {
                    self.at = At::Played(node);
                    return Advance::Audio { node, stream, fired };
                }
                NodeKind::Random => return self.end(WalkEnd::Random),
                NodeKind::FireEvent { event } => fired.push(event),
                NodeKind::Head | NodeKind::End => {}
            }
            let Some(next) = graph.next(node, value) else {
                let end = if current.kind == NodeKind::End { WalkEnd::EndNode } else { WalkEnd::DeadEnd };
                return self.end(end);
            };
            node = next;
        }
        self.end(WalkEnd::Loop)
    }

    fn end(&mut self, end: WalkEnd) -> Advance {
        self.at = At::Ended(end);
        Advance::End(end)
    }
}
