//! The career graph's node states: the list that follows the settings' `u32` count, one `{u32 node hash, u32 flags,
//! u32 best result, u16, u16}` record per event node (`docs/formats/career-save.md`, "Node state list").
//!
//! A node is keyed by the hash of its event id text (`"16.1.0"`), so the lookups here take the id string and hash it
//! with [`node_hash`]. [`NodeStates`] parses the 4,804-byte block and writes it back with the zero tail kept. The
//! records stay in file order and a new node is appended.

use super::NODE_LIST_LEN;
use crate::LayoutError;
use crate::bytes::{put_u16, put_u32, u16_at, u32_at};
use blackbox_attrib::vlt_hash;

/// The most records the list holds (the block has room for 300 of the 16-byte records).
pub const NODE_CAP: usize = 300;
/// Size of one record on disk.
const RECORD_LEN: usize = 16;

/// The key of a node: `vlt_hash` (Jenkins lookup2, initval `0xABCDEF00`) of the event id text. The initval is the one
/// the attribute database uses. It is confirmed on the three development saves: 128 to 132 of their records equal the
/// hashes of ids such as `1.2.3` and `16.2.1`, and the other candidates (`bstring_hash` raw and upper case, lookup2
/// with initval 0) equal none of them.
pub fn node_hash(event_id: &str) -> u32 {
    vlt_hash(event_id)
}

/// The state of one node: its flags, its best result and the two u16 values after the result. The meaning of `a`
/// and `b` is not known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NodeState {
    /// The state bits; see the associated constants. Unknown bits are kept as they are.
    pub flags: u32,
    /// The best result: a float time or score, or a u32 for one race kind. Kept as the raw u32.
    pub best: u32,
    /// The first u16 after the best result. Its meaning is not known.
    pub a: u16,
    /// The second u16 after the best result. Its meaning is not known.
    pub b: u16,
}

impl NodeState {
    /// Bit 0 (`0x1`): won outside the career.
    pub const WON_OUTSIDE_CAREER: u32 = 0x1;
    /// Bit 1 (`0x2`): won in the career. The completion test of story nodes such as `16.2.1` and `1.2.3`.
    pub const WON_IN_CAREER: u32 = 0x2;
    /// Bit 2 (`0x4`): unlocked in the career.
    pub const UNLOCKED_IN_CAREER: u32 = 0x4;
    /// Bit 3 (`0x8`): unlocked for the quick race.
    pub const UNLOCKED_QUICK_RACE: u32 = 0x8;
    /// Bit 4 (`0x10`): meaning not confirmed. Set on most nodes of an early save.
    pub const UNCONFIRMED: u32 = 0x10;

    /// True when the node was won in the career (bit 1).
    pub fn won_in_career(&self) -> bool {
        self.flags & Self::WON_IN_CAREER != 0
    }

    /// True when the node was won, in the career or outside it (bit 0 or bit 1).
    pub fn won(&self) -> bool {
        self.flags & (Self::WON_OUTSIDE_CAREER | Self::WON_IN_CAREER) != 0
    }

    /// True when the node is unlocked for the career or for the quick race (bit 2 or bit 3).
    pub fn unlocked(&self) -> bool {
        self.flags & (Self::UNLOCKED_IN_CAREER | Self::UNLOCKED_QUICK_RACE) != 0
    }
}

/// One record of the list: the node's key and its state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeRecord {
    /// The node key, [`node_hash`] of its event id.
    pub hash: u32,
    /// The node's state.
    pub state: NodeState,
}

/// Why a node change was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NodeListError {
    /// The list already holds [`NODE_CAP`] records, so a new node cannot get one.
    #[error("the node state list is full (at most 300 records)")]
    Full,
}

/// The node state list, parsed. Records are kept in file order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NodeStates {
    records: Vec<NodeRecord>,
}

impl NodeStates {
    /// Reads the list from its block: the `u32` count, then that many records. A count above [`NODE_CAP`] is read as
    /// the cap, and a block too short for the count is read as far as it goes. The bytes after the records are not
    /// read.
    pub fn parse(block: &[u8]) -> NodeStates {
        if block.len() < 4 {
            return NodeStates::default();
        }
        let fits = (block.len() - 4) / RECORD_LEN;
        let count = (u32_at(block, 0) as usize).min(NODE_CAP).min(fits);
        let records = (0..count).map(|i| read_record(block, 4 + i * RECORD_LEN)).collect();
        NodeStates { records }
    }

    /// The records, in file order.
    pub fn records(&self) -> &[NodeRecord] {
        &self.records
    }

    /// The number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// True when the list has no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The state of the node with this event id, or `None` when the list has no record for it.
    pub fn get(&self, event_id: &str) -> Option<NodeState> {
        let hash = node_hash(event_id);
        self.records.iter().find(|record| record.hash == hash).map(|record| record.state)
    }

    /// Sets the node's won-in-career bit (`0x2`), adding its record when it has none. Other bits and the stored
    /// best result stay as they are. Fails with [`NodeListError::Full`] only when a new record would pass the cap.
    pub fn set_won_in_career(&mut self, event_id: &str) -> Result<(), NodeListError> {
        self.update(event_id, |state| state.flags |= NodeState::WON_IN_CAREER)
    }

    /// Unlocks the node for the career (`0x4`). When `quick_race` is true the node is also unlocked for the quick race
    /// (`0x8`): the game leaves that bit out for races that are not in the quick race lists. Adds the record when it
    /// has none, and fails with [`NodeListError::Full`] only in that case.
    pub fn unlock(&mut self, event_id: &str, quick_race: bool) -> Result<(), NodeListError> {
        let bits = if quick_race {
            NodeState::UNLOCKED_IN_CAREER | NodeState::UNLOCKED_QUICK_RACE
        } else {
            NodeState::UNLOCKED_IN_CAREER
        };
        self.update(event_id, |state| state.flags |= bits)
    }

    /// Applies `change` to the node's state, adding a zeroed record first when the node has none.
    fn update(&mut self, event_id: &str, change: impl FnOnce(&mut NodeState)) -> Result<(), NodeListError> {
        let hash = node_hash(event_id);
        if let Some(record) = self.records.iter_mut().find(|record| record.hash == hash) {
            change(&mut record.state);
            return Ok(());
        }
        if self.records.len() >= NODE_CAP {
            return Err(NodeListError::Full);
        }
        let mut state = NodeState::default();
        change(&mut state);
        self.records.push(NodeRecord { hash, state });
        Ok(())
    }

    /// Writes the list into `out`, which must hold at least [`NODE_LIST_LEN`] bytes: the count, the records, then
    /// zeros up to [`NODE_LIST_LEN`]. Bytes after that are left alone.
    pub fn write(&self, out: &mut [u8]) -> Result<(), LayoutError> {
        LayoutError::check("node state list", out, NODE_LIST_LEN)?;
        self.encode(&mut out[..NODE_LIST_LEN]);
        Ok(())
    }

    /// Writes the list into a block of exactly [`NODE_LIST_LEN`] bytes. The caller checks the length.
    pub(super) fn encode(&self, out: &mut [u8]) {
        out[..NODE_LIST_LEN].fill(0);
        let kept = self.records.len().min(NODE_CAP);
        put_u32(out, 0, kept as u32);
        for (i, record) in self.records.iter().take(NODE_CAP).enumerate() {
            write_record(out, 4 + i * RECORD_LEN, record);
        }
    }
}

/// Reads the record at `o` (the caller checked that it fits).
fn read_record(block: &[u8], o: usize) -> NodeRecord {
    NodeRecord {
        hash: u32_at(block, o),
        state: NodeState {
            flags: u32_at(block, o + 4),
            best: u32_at(block, o + 8),
            a: u16_at(block, o + 12),
            b: u16_at(block, o + 14),
        },
    }
}

/// Writes the record at `o` (the caller checked that it fits).
fn write_record(out: &mut [u8], o: usize, record: &NodeRecord) {
    put_u32(out, o, record.hash);
    put_u32(out, o + 4, record.state.flags);
    put_u32(out, o + 8, record.state.best);
    put_u16(out, o + 12, record.state.a);
    put_u16(out, o + 14, record.state.b);
}

#[cfg(test)]
mod tests {
    use super::super::CareerSettings;
    use super::*;

    /// A block of the given records, with the count set and every other byte at `fill`.
    fn block_with(records: &[NodeRecord], fill: u8) -> Vec<u8> {
        let mut block = vec![fill; NODE_LIST_LEN];
        put_u32(&mut block, 0, records.len() as u32);
        for (i, record) in records.iter().enumerate() {
            write_record(&mut block, 4 + i * RECORD_LEN, record);
        }
        block
    }

    #[test]
    fn hashes_are_the_attribute_vault_hash_of_the_id() {
        assert_eq!(node_hash("16.1.0"), 0xB04A_C9BC);
        assert_eq!(node_hash("16.2.1"), 0xA684_F880);
        assert_eq!(node_hash("1.2.3"), 0xA283_EC6E);
    }

    #[test]
    fn an_empty_block_is_an_empty_list() {
        let nodes = NodeStates::parse(&[0u8; NODE_LIST_LEN]);
        assert!(nodes.is_empty());
        assert_eq!(nodes.get("16.1.0"), None);
        assert_eq!(NodeStates::parse(&[]), NodeStates::default());
    }

    #[test]
    fn records_are_read_at_the_documented_offsets() {
        let won = NodeState { flags: 0x1E, best: 0x4200_0000, a: 7, b: 9 };
        let block = block_with(&[NodeRecord { hash: node_hash("1.2.3"), state: won }], 0);
        let nodes = NodeStates::parse(&block);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes.get("1.2.3"), Some(won));
        assert_eq!(nodes.records()[0].hash, 0xA283_EC6E);
        let state = nodes.get("1.2.3").unwrap();
        assert!(state.won_in_career() && state.won() && state.unlocked());
    }

    #[test]
    fn parse_and_write_round_trip_the_block() {
        let records = [
            NodeRecord { hash: node_hash("16.1.0"), state: NodeState { flags: 0x2, best: 0, a: 0, b: 0 } },
            NodeRecord { hash: node_hash("16.2.1"), state: NodeState { flags: 0x1D, best: 3, a: 1, b: 2 } },
        ];
        let block = block_with(&records, 0);
        let mut written = vec![0u8; NODE_LIST_LEN];
        NodeStates::parse(&block).write(&mut written).unwrap();
        assert_eq!(written, block);
    }

    #[test]
    fn set_won_in_career_adds_a_record_and_keeps_the_others() {
        let mut nodes = NodeStates::default();
        nodes.set_won_in_career("16.2.1").unwrap();
        let state = nodes.get("16.2.1").unwrap();
        assert!(state.won_in_career() && state.won() && !state.unlocked());
        assert_eq!(state.flags, NodeState::WON_IN_CAREER);

        // Set again on a record that has other bits and a best result: only bit 1 is added.
        let record =
            NodeRecord { hash: node_hash("1.2.3"), state: NodeState { flags: 0x14, best: 12_345, a: 4, b: 5 } };
        let mut nodes = NodeStates::parse(&block_with(&[record], 0));
        nodes.set_won_in_career("1.2.3").unwrap();
        assert_eq!(nodes.get("1.2.3"), Some(NodeState { flags: 0x16, best: 12_345, a: 4, b: 5 }));
    }

    #[test]
    fn unlock_sets_career_bit_and_quick_race_bit_on_request() {
        let mut nodes = NodeStates::default();
        nodes.unlock("1.2.3", false).unwrap();
        assert_eq!(nodes.get("1.2.3").unwrap().flags, NodeState::UNLOCKED_IN_CAREER);
        assert!(nodes.get("1.2.3").unwrap().unlocked());
        assert!(!nodes.get("1.2.3").unwrap().won());

        nodes.unlock("16.2.1", true).unwrap();
        let state = nodes.get("16.2.1").unwrap();
        assert_eq!(state.flags, NodeState::UNLOCKED_IN_CAREER | NodeState::UNLOCKED_QUICK_RACE);
        assert!(state.unlocked() && !state.won_in_career());
    }

    #[test]
    fn the_cap_refuses_a_new_node_but_not_an_existing_one() {
        let mut nodes = NodeStates::default();
        for i in 0..NODE_CAP {
            nodes.unlock(&format!("test.{i}"), false).unwrap();
        }
        assert_eq!(nodes.len(), NODE_CAP);
        assert_eq!(nodes.set_won_in_career("test.300"), Err(NodeListError::Full));
        assert_eq!(nodes.get("test.300"), None);
        assert_eq!(nodes.len(), NODE_CAP);
        nodes.set_won_in_career("test.5").unwrap();
        assert!(nodes.get("test.5").unwrap().won_in_career());
    }

    #[test]
    fn a_count_above_the_cap_is_read_as_the_cap() {
        let mut block = vec![0u8; NODE_LIST_LEN];
        put_u32(&mut block, 0, u32::MAX);
        assert_eq!(NodeStates::parse(&block).len(), NODE_CAP);
    }

    #[test]
    fn write_keeps_the_block_size_and_zeroes_the_tail() {
        let mut nodes = NodeStates::default();
        nodes.unlock("1.2.3", true).unwrap();
        let mut out = vec![0xEEu8; NODE_LIST_LEN + 8];
        nodes.write(&mut out).unwrap();
        assert_eq!(&out[NODE_LIST_LEN..], &[0xEE; 8], "bytes past the block are left alone");
        assert!(out[4 + RECORD_LEN..NODE_LIST_LEN].iter().all(|&b| b == 0), "the tail after one record is zero");
        assert_eq!(&out[..4], &1u32.to_le_bytes());
    }

    #[test]
    fn write_refuses_a_short_buffer() {
        let nodes = NodeStates::default();
        let mut out = vec![0u8; NODE_LIST_LEN - 1];
        let err = nodes.write(&mut out).unwrap_err();
        assert_eq!((err.need, err.got), (NODE_LIST_LEN, NODE_LIST_LEN - 1));
    }

    #[test]
    fn career_settings_hand_the_list_over_and_back() {
        let mut settings = CareerSettings::new_career();
        assert!(settings.nodes().is_empty());
        let mut nodes = settings.nodes();
        nodes.set_won_in_career("16.1.0").unwrap();
        nodes.unlock("16.2.1", true).unwrap();
        settings.set_nodes(&nodes);
        assert_eq!(settings.node_states.len(), NODE_LIST_LEN);
        assert_eq!(settings.nodes(), nodes);
        assert!(settings.nodes().get("16.1.0").unwrap().won_in_career());
    }
}
