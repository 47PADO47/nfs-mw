//! The node records of the music graph.

use crate::bytes::{u16_le, u32_le};
use crate::error::Result;

/// One transition: the node to go to while the control value lies in `lo..=hi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    pub lo: i8,
    pub hi: i8,
    /// Index of the target node, or a negative number for "none" (the track ends).
    pub target: i16,
}

impl Transition {
    /// How far `value` is from the range (0 when inside it).
    fn distance(&self, value: i16) -> i16 {
        let (lo, hi) = (i16::from(self.lo), i16::from(self.hi));
        (lo - value).max(value - hi).max(0)
    }
}

/// What a node is, from the low 16 bits of its first word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// Plays the whole of one stream (index into the stream table of the `.mpf`).
    Audio { stream: u32 },
    /// Start of a group of nodes; makes no sound.
    Head,
    /// End of a group; a track that reaches it with nothing to follow stops.
    End,
    /// Runs the event with this 24-bit id when reached.
    FireEvent { event: u32 },
    /// Picks its next node at random (not used by Most Wanted's file).
    Random,
}

/// A node of the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub controller: u8,
    /// Section the node belongs to (songs, pursuit sets, ambience are different sections).
    pub section: u8,
    /// Bits 27 to 31 of the first word; the meaning is not known.
    pub repeat: u8,
    /// Router number, 1-based; 0 for none.
    pub router: u16,
    /// Bits 17 to 19 of the second word; 1 means the control value is random.
    pub random: u8,
    pub beats: u8,
    pub bars: u8,
    /// The group head this node belongs to.
    pub part: u16,
    pub transitions: Vec<Transition>,
}

const ID_HEAD: u16 = 0;
const ID_RANDOM: u16 = 0xFFFE;
const ID_FIRE_EVENT: u16 = 0xFFFD;
const ID_END: u16 = 0xFFFF;

impl Node {
    /// Read the node at byte `at`.
    pub(super) fn parse(data: &[u8], at: usize) -> Result<Self> {
        let d0 = u32_le(data, at)?;
        let d1 = u32_le(data, at + 4)?;
        let d2 = u32_le(data, at + 8)?;
        let d3 = u32_le(data, at + 12)?;
        let kind = match (d0 & 0xFFFF) as u16 {
            ID_HEAD => NodeKind::Head,
            ID_END => NodeKind::End,
            ID_RANDOM => NodeKind::Random,
            ID_FIRE_EVENT => NodeKind::FireEvent { event: d3 & 0x00FF_FFFF },
            id => NodeKind::Audio { stream: u32::from(id) - 1 },
        };
        let count = ((d1 >> 12) & 0x1F) as usize;
        let transitions = (0..count)
            .map(|i| {
                let base = at + 16 + 4 * i;
                let lo_hi = u16_le(data, base)?;
                Ok(Transition {
                    lo: lo_hi as u8 as i8,
                    hi: (lo_hi >> 8) as u8 as i8,
                    target: u16_le(data, base + 2)? as i16,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            kind,
            controller: ((d0 >> 16) & 0x1F) as u8,
            section: ((d0 >> 21) & 0x3F) as u8,
            repeat: (d0 >> 27) as u8,
            router: (d1 & 0xFFF) as u16,
            random: ((d1 >> 17) & 7) as u8,
            beats: ((d1 >> 20) & 0xF) as u8,
            bars: (d1 >> 24) as u8,
            part: (d2 & 0xFFFF) as u16,
            transitions,
        })
    }

    /// The transition taken for control value `value` (0 to 127): the first whose range holds it, else the one
    /// nearest to it (the first on a tie). `None` when the node has no transitions.
    pub fn select(&self, value: u8) -> Option<Transition> {
        let value = i16::from(value);
        let first = self.transitions.iter().find(|t| t.distance(value) == 0);
        first.or_else(|| self.transitions.iter().min_by_key(|t| t.distance(value))).copied()
    }
}
