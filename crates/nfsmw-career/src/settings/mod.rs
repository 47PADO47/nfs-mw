//! The career settings: current car, stage, cash, flags, the SMS inbox and the serial, plus the parts of the
//! 23,033-byte settings block this slice keeps verbatim (`docs/formats/career-save.md` section 3,
//! `docs/formats/career-state.md` section 3).
//!
//! The block is packed without padding. [`CareerSettings::from_block_bytes`] and
//! [`CareerSettings::to_block_bytes`] are exact inverses: the gameplay-state blob and the shop unlock table are
//! not decoded yet and travel as raw bytes. The node state list is read and written through [`NodeStates`]
//! ([`CareerSettings::nodes`], [`CareerSettings::set_nodes`]). The marker inventory is decoded by
//! [`crate::economy::markers`].

mod flags;
mod new_career;
mod nodes;
mod sms;

pub use flags::Flags;
pub use new_career::{
    NewCareer, SERIAL_NUMBER_OFFSET, SERIAL_NUMBER_RANGE, SERIAL_TEXT_MAX, SerialRng, format_serial, generate_serial,
    new_career_with,
};
pub use nodes::{NODE_CAP, NodeListError, NodeRecord, NodeState, NodeStates, node_hash};
pub use sms::{SMS_INVALID_ID, SMS_SLOTS, SMS_TABLE_LEN, SmsInbox, SmsSlot, SmsState};

use crate::LayoutError;
use crate::bytes::{array_at, put_u16, put_u32, u16_at, u32_at};
use save_store::digest::md5;

/// Size of the settings block in the save (file `0x34..0x5A2D`).
pub const SETTINGS_BLOCK_LEN: usize = 23_033;
/// Offset of the settings block inside the profile buffer (the buffer starts with a 16-byte version field).
pub const PROFILE_SETTINGS_OFFSET: usize = 0x10;
/// Size of the gameplay-state blob that opens the block.
pub const GAMEPLAY_BLOB_LEN: usize = 0x4000;
/// Size of the node state list (`u32` count, then 300 entries of 16 bytes).
pub const NODE_LIST_LEN: usize = 4_804;
/// Size of the shop part-unlock table (57 entries of 8 bytes).
pub const SHOP_UNLOCK_LEN: usize = 456;
/// Size of the marker and part-unlock inventory (63 entries of 12 bytes, `career-economy-flows.md` 6.5). This is not
/// the popup queue, which is a separate table that is not saved.
pub const INVENTORY_LEN: usize = 756;

/// Block offsets of the packed fields (file offset minus `0x34`).
mod at {
    pub const CAR: usize = 0x4000;
    pub const STAGE: usize = 0x4004;
    pub const CASH: usize = 0x4005;
    pub const UNKNOWN: usize = 0x4009;
    pub const FLAGS: usize = 0x400B;
    pub const SMS_TABLE: usize = 0x400F;
    pub const SMS_COUNTER: usize = 0x4267;
    pub const SERIAL: usize = 0x4269;
    pub const NODES: usize = 0x4279;
    pub const SHOP: usize = 0x553D;
    pub const INVENTORY: usize = 0x5705;
}

/// The handle of the preset car `M3GTRCAREERSTART`, the car of a new career (`career-flow.md` section 3).
pub const STARTER_CAR_HANDLE: u32 = 0x38A3_C42C;
/// The name of the starter preset (for display in logs and tools; the handle is what the save stores).
pub const STARTER_PRESET_NAME: &str = "M3GTRCAREERSTART";
/// The stage of a new career: the prologue, before the first of the 15 Blacklist rivals.
pub const PROLOGUE_STAGE: u8 = 16;
/// The first Blacklist stage, reached by the first safe house visit.
pub const FIRST_RIVAL_STAGE: u8 = 15;
/// The graph node a new career activates: the opening D-Day race.
pub const PROLOGUE_START_NODE: &str = "16.1.0";
/// The cash a first profile gets when an Underground 2 profile folder exists (`career-flow.md` section 3). The
/// folder check itself belongs to the caller.
pub const UG2_BONUS_CASH: u32 = 10_000;

/// The career settings of a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareerSettings {
    /// Handle of the current car's `FECarRecord` in the stable.
    pub car_handle: u32,
    /// The Blacklist rank being challenged (`CurrentBin`): 16 before the Blacklist, then 15 down to 1.
    pub stage: u8,
    /// Cash. Spent with [`CareerSettings::spend_cash`], which clamps at 0.
    pub cash: u32,
    /// The u16 at settings `+0x10`; its meaning is not known (0 in two saves, 30,372 in a third).
    pub unknown_0x10: u16,
    /// The flags word.
    pub flags: Flags,
    /// The 150-slot SMS inbox and its counter.
    pub sms: SmsInbox,
    /// The profile serial text (`MW-<n>-<4 letters>XXX`), zero padded.
    pub serial: [u8; 16],
    /// The gameplay-manager snapshot (`Game` blob), raw; its first 16 bytes are the MD5 of the rest.
    pub gameplay_blob: Box<[u8; GAMEPLAY_BLOB_LEN]>,
    /// The career graph's node state list, raw. Read it with [`CareerSettings::nodes`].
    pub node_states: Box<[u8; NODE_LIST_LEN]>,
    /// The shop part-unlock tiers, raw.
    pub shop_unlocks: Box<[u8; SHOP_UNLOCK_LEN]>,
    /// The marker and part-unlock inventory at file `0x5739`, raw: 63 entries of `{u32 type, u32 aux, u32 flag}`.
    /// Decode it with [`crate::economy::markers::MarkerInventory::from_table`]. Not the popup queue.
    pub inventory: Box<[u8; INVENTORY_LEN]>,
}

impl CareerSettings {
    /// The settings of a new career (`career-flow.md` section 3): the starter car, flags with only "career
    /// started", stage 16, cash 0, an inbox with every slot numbered and the counter at 0.
    ///
    /// The inbox keeps `id = index` for all 150 slots: the game then marks slots whose `SMS_MESSAGE_<n>` label is
    /// missing with 0xFF, which needs the install's language table; call
    /// [`SmsInbox::invalidate_missing`] on `sms` for that. The serial is left empty (the original draws it from a
    /// generator seeded with `0x19B3`; use [`CareerSettings::set_serial`]). The gameplay blob, node list, shop
    /// table and inventory start zeroed: the gameplay blob is the progress slice's to build, and the original
    /// cannot load a zeroed one.
    pub fn new_career() -> CareerSettings {
        CareerSettings {
            car_handle: STARTER_CAR_HANDLE,
            stage: PROLOGUE_STAGE,
            cash: 0,
            unknown_0x10: 0,
            flags: Flags::CAREER_STARTED,
            sms: SmsInbox::new(),
            serial: [0; 16],
            gameplay_blob: Box::new([0; GAMEPLAY_BLOB_LEN]),
            node_states: Box::new([0; NODE_LIST_LEN]),
            shop_unlocks: Box::new([0; SHOP_UNLOCK_LEN]),
            inventory: Box::new([0; INVENTORY_LEN]),
        }
    }

    /// Reads the settings from a settings block of [`SETTINGS_BLOCK_LEN`] bytes (extra bytes are ignored).
    pub fn from_block_bytes(block: &[u8]) -> Result<CareerSettings, LayoutError> {
        LayoutError::check("career settings block", block, SETTINGS_BLOCK_LEN)?;
        Ok(CareerSettings {
            car_handle: u32_at(block, at::CAR),
            stage: block[at::STAGE],
            cash: u32_at(block, at::CASH),
            unknown_0x10: u16_at(block, at::UNKNOWN),
            flags: Flags(u32_at(block, at::FLAGS)),
            sms: SmsInbox::from_parts(
                &block[at::SMS_TABLE..at::SMS_TABLE + SMS_TABLE_LEN],
                &block[at::SMS_COUNTER..at::SMS_COUNTER + 2],
            ),
            serial: array_at(block, at::SERIAL),
            gameplay_blob: Box::new(array_at(block, 0)),
            node_states: Box::new(array_at(block, at::NODES)),
            shop_unlocks: Box::new(array_at(block, at::SHOP)),
            inventory: Box::new(array_at(block, at::INVENTORY)),
        })
    }

    /// Reads the settings from a whole profile buffer (the 63,560 bytes that follow the 36-byte file prefix).
    pub fn from_profile_bytes(profile: &[u8]) -> Result<CareerSettings, LayoutError> {
        LayoutError::check("profile buffer", profile, PROFILE_SETTINGS_OFFSET + SETTINGS_BLOCK_LEN)?;
        CareerSettings::from_block_bytes(&profile[PROFILE_SETTINGS_OFFSET..])
    }

    /// The packed settings block, [`SETTINGS_BLOCK_LEN`] bytes.
    pub fn to_block_bytes(&self) -> Vec<u8> {
        let mut block = vec![0u8; SETTINGS_BLOCK_LEN];
        block[..GAMEPLAY_BLOB_LEN].copy_from_slice(&self.gameplay_blob[..]);
        put_u32(&mut block, at::CAR, self.car_handle);
        block[at::STAGE] = self.stage;
        put_u32(&mut block, at::CASH, self.cash);
        put_u16(&mut block, at::UNKNOWN, self.unknown_0x10);
        put_u32(&mut block, at::FLAGS, self.flags.0);
        let (table, rest) = block[at::SMS_TABLE..].split_at_mut(SMS_TABLE_LEN);
        self.sms.write_parts(table, &mut rest[..2]);
        block[at::SERIAL..at::SERIAL + 16].copy_from_slice(&self.serial);
        block[at::NODES..at::NODES + NODE_LIST_LEN].copy_from_slice(&self.node_states[..]);
        block[at::SHOP..at::SHOP + SHOP_UNLOCK_LEN].copy_from_slice(&self.shop_unlocks[..]);
        block[at::INVENTORY..at::INVENTORY + INVENTORY_LEN].copy_from_slice(&self.inventory[..]);
        block
    }

    /// Writes the settings block into a profile buffer at [`PROFILE_SETTINGS_OFFSET`].
    pub fn write_into_profile(&self, profile: &mut [u8]) -> Result<(), LayoutError> {
        LayoutError::check("profile buffer", profile, PROFILE_SETTINGS_OFFSET + SETTINGS_BLOCK_LEN)?;
        profile[PROFILE_SETTINGS_OFFSET..PROFILE_SETTINGS_OFFSET + SETTINGS_BLOCK_LEN]
            .copy_from_slice(&self.to_block_bytes());
        Ok(())
    }

    /// Sets the serial text, truncated to 16 bytes and zero padded.
    pub fn set_serial(&mut self, text: &str) {
        let mut serial = [0u8; 16];
        let n = text.len().min(16);
        serial[..n].copy_from_slice(&text.as_bytes()[..n]);
        self.serial = serial;
    }

    /// The serial text up to the first zero byte (lossy for non-UTF-8 bytes).
    pub fn serial_text(&self) -> String {
        let end = self.serial.iter().position(|&b| b == 0).unwrap_or(16);
        String::from_utf8_lossy(&self.serial[..end]).into_owned()
    }

    /// True when the career has been started (flag bit 0): the profile resumes instead of starting anew.
    pub fn has_career(&self) -> bool {
        self.flags.contains(Flags::CAREER_STARTED)
    }

    /// True when the cash covers `amount`.
    pub fn can_afford(&self, amount: u32) -> bool {
        self.cash >= amount
    }

    /// Spends `amount` the way the original does: `cash = max(cash - amount, 0)`, never negative. Returns false
    /// when the cash did not cover the amount (the clamp was used), so a caller that must refuse can check first
    /// with [`CareerSettings::can_afford`].
    pub fn spend_cash(&mut self, amount: u32) -> bool {
        let covered = self.cash >= amount;
        self.cash = self.cash.saturating_sub(amount);
        covered
    }

    /// Adds `amount` to the cash and returns the new balance. No cash cap was found in the original; the sum
    /// saturates at `u32::MAX` instead of wrapping (conservative, unconfirmed).
    pub fn add_cash(&mut self, amount: u32) -> u32 {
        self.cash = self.cash.saturating_add(amount);
        self.cash
    }

    /// True when the blob's leading MD5 matches the rest of the blob (bytes `0x10..0x4000`).
    pub fn gameplay_digest_ok(&self) -> bool {
        self.gameplay_blob[..16] == md5(&self.gameplay_blob[16..])
    }

    /// Recomputes the blob's leading MD5. Call it after changing the blob; the other methods never touch it.
    pub fn refresh_gameplay_digest(&mut self) {
        let digest = md5(&self.gameplay_blob[16..]);
        self.gameplay_blob[..16].copy_from_slice(&digest);
    }

    /// The node state list, parsed from the raw block. Edit a copy and hand it back with
    /// [`CareerSettings::set_nodes`].
    pub fn nodes(&self) -> NodeStates {
        NodeStates::parse(&self.node_states[..])
    }

    /// Replaces the raw node block with `nodes`: the count, the records and a zero tail, the block size unchanged.
    pub fn set_nodes(&mut self, nodes: &NodeStates) {
        nodes.encode(&mut self.node_states[..]);
    }
}

#[cfg(test)]
mod tests;
