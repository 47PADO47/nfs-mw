//! The respawn marker pair at the end of the gameplay blob (`docs/formats/career-save.md` 3.1, item 8;
//! `docs/specs/career-safehouse-2.md` 9.4): the key of the last visited zone's respawn marker at the layout's
//! `references`, and the last visited safe house's 16 bytes further on. The original writes both with the blob and, on
//! a load, takes each only when its collection exists in the loaded data.

use super::{GAME_MAGIC, MilestoneBlock};
use crate::LayoutError;
use crate::bytes::{put_u32, u32_at};

/// The step from the first key of the pair to the second.
const SAFE_HOUSE_STEP: usize = 0x10;

/// The pair as the blob stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RespawnKeys {
    /// The respawn marker of the last visited zone of any kind (`+0x108` of the gameplay manager).
    pub home: u32,
    /// The respawn marker of the last visited safe house (`+0x10C`).
    pub safe_house: u32,
}

impl MilestoneBlock {
    /// The respawn pair, or `None` for a blob that is not a written gameplay blob (no `Game` tag) or cannot be laid out.
    pub fn respawn_keys(&self) -> Option<RespawnKeys> {
        if self.game_magic() != GAME_MAGIC {
            return None;
        }
        let at = self.layout().ok()?.references;
        Some(RespawnKeys { home: u32_at(self.bytes(), at), safe_house: u32_at(self.bytes(), at + SAFE_HOUSE_STEP) })
    }

    /// Stores the respawn pair and refreshes the digest. A blob without the `Game` tag is left alone (`Ok(false)`): the
    /// pair has no place in it until the rest of the blob is written.
    pub fn set_respawn_keys(&mut self, keys: RespawnKeys) -> Result<bool, LayoutError> {
        if self.game_magic() != GAME_MAGIC {
            return Ok(false);
        }
        let at = self.layout()?.references;
        put_u32(&mut self.raw[..], at, keys.home);
        put_u32(&mut self.raw[..], at + SAFE_HOUSE_STEP, keys.safe_house);
        self.refresh_digest();
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::milestones_block::BLOCK_LEN;

    /// A written blob with no objects, slots or arrays: the pair starts right after the 0x80-byte header.
    fn written() -> MilestoneBlock {
        let mut raw = vec![0u8; BLOCK_LEN];
        put_u32(&mut raw, 0x10, GAME_MAGIC);
        put_u32(&mut raw, 0x14, 8);
        let mut block = MilestoneBlock::from_bytes(&raw).unwrap();
        block.refresh_digest();
        block
    }

    #[test]
    fn the_pair_is_read_and_written_where_the_layout_puts_it_and_the_digest_follows() {
        let mut block = written();
        let at = block.layout().unwrap().references;
        let keys = RespawnKeys { home: 0x1234_5678, safe_house: 0x9ABC_DEF0 };
        assert_eq!(block.set_respawn_keys(keys), Ok(true));
        assert_eq!(block.respawn_keys(), Some(keys));
        assert_eq!(u32_at(block.bytes(), at + 0x10), 0x9ABC_DEF0, "the safe house key is 16 bytes after the first");
        assert!(block.digest_ok(), "the digest covers the new keys");
    }

    #[test]
    fn a_blank_blob_has_no_pair_and_is_not_written() {
        let mut block = MilestoneBlock::from_bytes(&vec![0u8; BLOCK_LEN]).unwrap();
        assert_eq!(block.respawn_keys(), None);
        assert_eq!(block.set_respawn_keys(RespawnKeys { home: 1, safe_house: 1 }), Ok(false));
        assert!(block.bytes().iter().all(|&b| b == 0), "a blank blob stays blank");
    }
}
