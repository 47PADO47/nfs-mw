//! Following a track through the graph without playing it.

use super::{Graph, NodeKind};
use crate::error::{Error, Result};

/// Why a walk stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkEnd {
    /// It reached an end node with nothing to follow.
    EndNode,
    /// A node had no transition, or its target was "none".
    DeadEnd,
    /// It came back to a node it had already visited (a looping track).
    Loop,
    /// It reached a node that picks its next node at random, which a walk cannot predict.
    Random,
}

/// The nodes a track passes, in order, and how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walk {
    /// Every node visited, the start included, control nodes too.
    pub nodes: Vec<usize>,
    pub end: WalkEnd,
}

impl Walk {
    /// The audio nodes of the walk, in order: the stream each plays is [`NodeKind::Audio::stream`].
    pub fn audio<'g>(&'g self, graph: &'g Graph) -> impl Iterator<Item = usize> + 'g {
        self.nodes.iter().copied().filter(|&n| matches!(graph.nodes[n].kind, NodeKind::Audio { .. }))
    }
}

impl Graph {
    /// Follow the track that starts at `start` with a constant control value, the way the player would while
    /// nothing changes the value. Events named by fire-event nodes are not run: the walk goes on to the node
    /// after them.
    pub fn walk(&self, start: usize, value: u8) -> Result<Walk> {
        if start >= self.nodes.len() {
            return Err(Error::NoSuchEntry(start));
        }
        let mut seen = vec![false; self.nodes.len()];
        let mut nodes = Vec::new();
        let mut current = start;
        loop {
            if std::mem::replace(&mut seen[current], true) {
                return Ok(Walk { nodes, end: WalkEnd::Loop });
            }
            nodes.push(current);
            let node = &self.nodes[current];
            if node.kind == NodeKind::Random {
                return Ok(Walk { nodes, end: WalkEnd::Random });
            }
            let Some(next) = self.next(current, value) else {
                let end = if node.kind == NodeKind::End { WalkEnd::EndNode } else { WalkEnd::DeadEnd };
                return Ok(Walk { nodes, end });
            };
            current = next;
        }
    }
}
