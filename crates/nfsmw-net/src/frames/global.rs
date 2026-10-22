//! The global block of a snapshot (protocol.md section 6.3): what the HUD needs from the server beyond the own car.
//!
//! Layout, most significant bit first, 96 bits: session phase 4, heat level 4, busted fraction 8, cooldown fraction 8,
//! pursuit status 3, race place 5 (0 = none), lap 6, checkpoint 8, time of day 16 (minutes × 45, so 1/45 minute steps),
//! then 34 reserved zero bits.

use blackbox_net_core::bits::{BitReader, BitWriter};

use super::FrameError;
use crate::message::SessionPhase;

/// Bits of the global block, reserved bits included.
pub const GLOBAL_BLOCK_BITS: usize = 96;
/// Bits the defined fields use.
const USED_BITS: usize = 62;
/// Time-of-day codes per minute.
const TIME_STEPS_PER_MINUTE: f32 = 45.0;
/// Minutes in a day.
const DAY_MINUTES: f32 = 1440.0;

/// The receiving player's pursuit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PursuitStatus {
    #[default]
    None = 0,
    /// Police are chasing the player.
    Chased = 1,
    /// Out of sight; the cooldown meter runs.
    Cooldown = 2,
    Busted = 3,
    Evaded = 4,
}

impl PursuitStatus {
    fn from_code(code: u64) -> Option<Self> {
        match code {
            0 => Some(Self::None),
            1 => Some(Self::Chased),
            2 => Some(Self::Cooldown),
            3 => Some(Self::Busted),
            4 => Some(Self::Evaded),
            _ => None,
        }
    }
}

/// The global block: plain values for the HUD.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalBlock {
    pub phase: SessionPhase,
    /// Heat level, 0..15.
    pub heat: u8,
    /// Busted meter, 0..1 in 1/255 steps.
    pub busted: f32,
    /// Cooldown meter, 0..1 in 1/255 steps.
    pub cooldown: f32,
    pub pursuit: PursuitStatus,
    /// Race place from 1, 0 when not racing; at most 31.
    pub place: u8,
    /// Lap, 0..63.
    pub lap: u8,
    /// Checkpoint index, 0..255.
    pub checkpoint: u8,
    /// Minutes after midnight, under 1,440.
    pub time_of_day: f32,
}

impl Default for GlobalBlock {
    fn default() -> Self {
        Self {
            phase: SessionPhase::Lobby,
            heat: 0,
            busted: 0.0,
            cooldown: 0.0,
            pursuit: PursuitStatus::None,
            place: 0,
            lap: 0,
            checkpoint: 0,
            time_of_day: 0.0,
        }
    }
}

impl GlobalBlock {
    /// Writes the block as [`GLOBAL_BLOCK_BITS`] bits. Counts are clamped to their fields; a non-finite meter or time is
    /// [`FrameError::BadValue`].
    pub fn write(&self, w: &mut BitWriter<'_>) -> Result<(), FrameError> {
        if w.remaining_bits() < GLOBAL_BLOCK_BITS {
            return Err(FrameError::BadLength);
        }
        let time = self.time_of_day.rem_euclid(DAY_MINUTES) * TIME_STEPS_PER_MINUTE;
        if !time.is_finite() {
            return Err(FrameError::BadValue);
        }
        let fields = [
            (u64::from(self.phase.code()), 4),
            (u64::from(self.heat.min(15)), 4),
            (u64::from(unit(self.busted)?), 8),
            (u64::from(unit(self.cooldown)?), 8),
            (self.pursuit as u64, 3),
            (u64::from(self.place.min(31)), 5),
            (u64::from(self.lap.min(63)), 6),
            (u64::from(self.checkpoint), 8),
            ((time.round() as u64).min(u64::from(u16::MAX)), 16),
            (0, (GLOBAL_BLOCK_BITS - USED_BITS) as u32),
        ];
        for (value, bits) in fields {
            w.write_bits(value, bits).map_err(|_| FrameError::BadLength)?;
        }
        Ok(())
    }

    /// Reads a block written by [`GlobalBlock::write`]. An unknown phase or pursuit code, a time past the day or a
    /// nonzero reserved bit is [`FrameError::BadBlock`].
    pub fn read(r: &mut BitReader<'_>) -> Result<Self, FrameError> {
        if r.remaining_bits() < GLOBAL_BLOCK_BITS {
            return Err(FrameError::BadLength);
        }
        let mut take = |bits: u32| r.read_bits(bits).map_err(|_| FrameError::BadLength);
        let phase = SessionPhase::from_code(take(4)? as u8).ok_or(FrameError::BadBlock)?;
        let heat = take(4)? as u8;
        let busted = take(8)? as f32 / 255.0;
        let cooldown = take(8)? as f32 / 255.0;
        let pursuit = PursuitStatus::from_code(take(3)?).ok_or(FrameError::BadBlock)?;
        let place = take(5)? as u8;
        let lap = take(6)? as u8;
        let checkpoint = take(8)? as u8;
        let time_of_day = take(16)? as f32 / TIME_STEPS_PER_MINUTE;
        if time_of_day >= DAY_MINUTES {
            return Err(FrameError::BadBlock);
        }
        if take((GLOBAL_BLOCK_BITS - USED_BITS) as u32)? != 0 {
            return Err(FrameError::Reserved);
        }
        Ok(Self { phase, heat, busted, cooldown, pursuit, place, lap, checkpoint, time_of_day })
    }
}

/// A 0..1 meter as 0..255.
fn unit(x: f32) -> Result<u8, FrameError> {
    if !x.is_finite() {
        return Err(FrameError::BadValue);
    }
    Ok((x.clamp(0.0, 1.0) * 255.0).round() as u8)
}
