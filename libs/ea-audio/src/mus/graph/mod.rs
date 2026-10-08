//! The node graph of an interactive-music map (`.mpf`, PathFinder).
//!
//! Nodes are bars of music (each plays one stream), or control points: group heads, ends, and nodes that fire an
//! event. A node's transitions name the node to go to for a control value, routers rewrite the choice, and
//! events are small programs (the one this reader interprets is the song start: branch to a node). Layout and
//! behaviour: `docs/specs/music-graph.md`.
//!
//! [`Graph::walk`] follows a song from its first node to its end; [`Mpf::chain`](super::Mpf::chain) adds the
//! streams and their lengths.

mod event;
mod node;
mod walk;

pub use event::{Action, Event, OP_BRANCH_TO};
pub use node::{Node, NodeKind, Transition};
pub use walk::{Walk, WalkEnd};

use crate::bytes::{u8_at, u16_le, u32_le};
use crate::error::{Error, Result};

/// A router entry: when the chosen target is `key`, go to `value` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouterEntry {
    pub key: u16,
    pub value: u16,
}

/// A named variable of the map and its initial value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub initial: u32,
}

/// The parsed graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graph {
    /// Index of the map in the game's event ids.
    pub project: u8,
    pub sections: u8,
    pub nodes: Vec<Node>,
    pub events: Vec<Event>,
    /// Router `r` (1-based, as a node names it) is `routers[r - 1]`.
    pub routers: Vec<Vec<RouterEntry>>,
    pub variables: Vec<Variable>,
}

/// Bytes per variable record.
const VARIABLE: usize = 20;
/// Bytes of the name in a variable record.
const VARIABLE_NAME: usize = 16;

impl Graph {
    /// Parse the graph of a version 5 map file.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.get(..4) != Some(b"xDFP") {
            return Err(Error::BadMagic { expected: "xDFP (little-endian PFDx)" });
        }
        if u8_at(data, 4)? != 5 {
            return Err(Error::Unsupported("mpf version other than 5"));
        }
        let project = u8_at(data, 0x0C)?;
        let sections = u8_at(data, 0x0E)?;
        let event_count = usize::from(u8_at(data, 0x0F)?);
        let router_count = usize::from(u8_at(data, 0x10)?);
        let variable_count = usize::from(u8_at(data, 0x11)?);
        let node_count = usize::from(u16_le(data, 0x12)?);

        let node_table = u32_le(data, 0x14)? as usize;
        let nodes = (0..node_count)
            .map(|i| Node::parse(data, usize::from(u16_le(data, node_table + 2 * i)?) * 4))
            .collect::<Result<Vec<_>>>()?;
        let event_table = u32_le(data, 0x1C)? as usize;
        let events = (0..event_count)
            .map(|i| Event::parse(data, usize::from(u16_le(data, event_table + 2 * i)?) * 4))
            .collect::<Result<Vec<_>>>()?;
        let variable_table = u32_le(data, 0x24)? as usize;
        let variables =
            (0..variable_count).map(|i| variable(data, variable_table + VARIABLE * i)).collect::<Result<Vec<_>>>()?;
        let routers = routers(data, u32_le(data, 0x28)? as usize, router_count)?;
        let graph = Self { project, sections, nodes, events, routers, variables };
        graph.check()?;
        Ok(graph)
    }

    /// Every transition and branch must name a node that exists (or none).
    fn check(&self) -> Result<()> {
        let count = self.nodes.len();
        let known = |n: i64| n < 0 || (n as usize) < count;
        let transitions_ok = self.nodes.iter().flat_map(|n| &n.transitions).all(|t| known(i64::from(t.target)));
        if !transitions_ok {
            return Err(Error::Corrupt("a transition names a node that does not exist"));
        }
        let branches_ok =
            self.events.iter().flat_map(|e| &e.actions).filter_map(Action::branch_node).all(|n| n < count);
        if !branches_ok {
            return Err(Error::Corrupt("an event branches to a node that does not exist"));
        }
        Ok(())
    }

    /// The event with the given id, comparing the low 24 bits only (a game id has a project bit above them).
    pub fn event(&self, id: u32) -> Option<&Event> {
        let id = id & 0x00FF_FFFF;
        self.events.iter().find(|e| e.id == id)
    }

    /// The node a song starts at, from the id of its event.
    pub fn song_start(&self, event_id: u32) -> Option<usize> {
        self.event(event_id)?.start_node()
    }

    /// The node that follows `from` for control value `value`: the selected transition, rewritten by the node's
    /// router. `None` when the node has no transition or the target is "none".
    pub fn next(&self, from: usize, value: u8) -> Option<usize> {
        let node = self.nodes.get(from)?;
        let chosen = node.select(value)?.target;
        if chosen < 0 {
            return None;
        }
        let mut target = chosen;
        let router = usize::from(node.router).checked_sub(1).and_then(|r| self.routers.get(r));
        for entry in router.into_iter().flatten().filter(|e| i32::from(e.key) == i32::from(chosen)) {
            target = entry.value as i16;
        }
        usize::try_from(target).ok().filter(|&t| t < self.nodes.len())
    }
}

fn variable(data: &[u8], at: usize) -> Result<Variable> {
    let name = data.get(at..at + VARIABLE_NAME).ok_or(Error::Truncated { offset: at as u64, needed: VARIABLE_NAME })?;
    let end = name.iter().position(|&b| b == 0).unwrap_or(VARIABLE_NAME);
    Ok(Variable {
        name: String::from_utf8_lossy(&name[..end]).into_owned(),
        initial: u32_le(data, at + VARIABLE_NAME)?,
    })
}

/// The routers: `count + 1` offsets (in `u32` units from the start of the file), router `r` holding the entries
/// from offset `r - 1` up to offset `r`.
fn routers(data: &[u8], table: usize, count: usize) -> Result<Vec<Vec<RouterEntry>>> {
    let offsets = (0..=count).map(|i| u32_le(data, table + 4 * i).map(|o| o as usize)).collect::<Result<Vec<_>>>()?;
    offsets
        .windows(2)
        .map(|w| {
            if w[1] < w[0] {
                return Err(Error::Corrupt("router offsets are not ascending"));
            }
            (w[0]..w[1])
                .map(|k| {
                    let word = u32_le(data, 4 * k)?;
                    Ok(RouterEntry { key: (word >> 16) as u16, value: word as u16 })
                })
                .collect()
        })
        .collect()
}
