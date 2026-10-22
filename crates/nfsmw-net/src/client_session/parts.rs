//! Snapshot parts waiting for their siblings (protocol.md section 6.3): parts arrive in any order; a snapshot is
//! complete when every part of its tick is in. A few ticks are open at once; an older incomplete one is dropped when a
//! newer one completes or the table is full.

use nfsmw_sim::Tick;

use crate::frames::{GlobalBlock, MAX_PARTS, SnapshotFrame};
use crate::schema::OwnBlock;

/// Snapshots assembled at once.
const OPEN: usize = 4;

/// One snapshot, with every part in.
#[derive(Debug, Clone, PartialEq)]
pub struct Assembled {
    pub tick: Tick,
    pub ack_input: u16,
    pub own: Option<OwnBlock>,
    pub global: Option<GlobalBlock>,
    /// The part bodies in part order.
    pub bodies: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
struct Open {
    tick: Tick,
    parts: u8,
    ack_input: u16,
    own: Option<OwnBlock>,
    global: Option<GlobalBlock>,
    bodies: Vec<Option<Vec<u8>>>,
}

/// The open snapshots.
#[derive(Debug, Clone, Default)]
pub struct PartBuffer {
    open: Vec<Open>,
    dropped: u64,
    conflicts: u64,
}

impl PartBuffer {
    /// Takes one part. Returns the snapshot when this part completes it. A part whose count disagrees with an earlier
    /// part of the same tick, or a duplicate part, is counted and dropped.
    pub fn accept(&mut self, frame: &SnapshotFrame<'_>) -> Option<Assembled> {
        let header = frame.header;
        if header.parts == 1 {
            return Some(Assembled {
                tick: frame.tick,
                ack_input: header.ack_input,
                own: frame.own,
                global: frame.global,
                bodies: vec![frame.body.to_vec()],
            });
        }
        let index = match self.open.iter().position(|open| open.tick == frame.tick) {
            Some(index) => index,
            None => self.open_new(frame),
        };
        let open = &mut self.open[index];
        let slot = usize::from(header.part);
        if open.parts != header.parts || open.bodies.get(slot).is_none_or(Option::is_some) {
            self.conflicts += 1;
            return None;
        }
        open.bodies[slot] = Some(frame.body.to_vec());
        if header.part == 0 {
            open.ack_input = header.ack_input;
            open.own = frame.own;
            open.global = frame.global;
        }
        if open.bodies.iter().any(Option::is_none) {
            return None;
        }
        let open = self.open.remove(index);
        // Older snapshots still open can no longer be newest: drop them.
        let before = self.open.len();
        self.open.retain(|other| other.tick > open.tick);
        self.dropped += (before - self.open.len()) as u64;
        Some(Assembled {
            tick: open.tick,
            ack_input: open.ack_input,
            own: open.own,
            global: open.global,
            bodies: open.bodies.into_iter().flatten().collect(),
        })
    }

    /// Forgets every open snapshot.
    pub fn clear(&mut self) {
        self.open.clear();
    }

    /// Incomplete snapshots given up.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Parts refused as duplicates or inconsistent.
    pub fn conflicts(&self) -> u64 {
        self.conflicts
    }

    fn open_new(&mut self, frame: &SnapshotFrame<'_>) -> usize {
        if self.open.len() >= OPEN {
            let oldest = self.open.iter().enumerate().min_by_key(|(_, open)| open.tick).map(|(index, _)| index);
            if let Some(oldest) = oldest {
                self.open.remove(oldest);
                self.dropped += 1;
            }
        }
        let parts = frame.header.parts.min(MAX_PARTS);
        self.open.push(Open {
            tick: frame.tick,
            parts,
            ack_input: frame.header.ack_input,
            own: None,
            global: None,
            bodies: vec![None; usize::from(parts)],
        });
        self.open.len() - 1
    }
}
