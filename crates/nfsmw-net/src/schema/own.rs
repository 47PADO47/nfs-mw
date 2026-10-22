//! The own-car block of a snapshot (protocol.md section 6.3): the receiving player's car, reduced authoritative state at
//! the acknowledged input tick, for reconciliation.
//!
//! Layout, most significant bit first, 256 bits in all: position 3 × 20, orientation 32 (smallest three), linear
//! velocity 3 × 14, angular velocity 3 × 10, rpm 10, gear 4, nitrous 7, boost 6, flags 8, cause 3, input report 24
//! (early, on time, late, missing: 5 bits each; mean earliness 4), then 30 reserved zero bits.

use blackbox_net_core::bits::{BitReader, BitWriter};
use nfsmw_sim::{EntityView, OwnState};

use super::quant::{self, clamped, pack_quat, unpack_quat, value};
use super::{GameSchema, POSITION_BITS, SchemaError};

/// Bits of the own block, reserved bits included.
pub const OWN_BLOCK_BITS: usize = 256;
/// Bits the defined fields use.
const USED_BITS: usize = 226;

/// Why the server moved the player's car against its prediction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum CorrectionCause {
    #[default]
    None = 0,
    Contact = 1,
    Reset = 2,
    Teleport = 3,
    Freeze = 4,
}

impl CorrectionCause {
    fn from_code(code: u64) -> Option<Self> {
        match code {
            0 => Some(Self::None),
            1 => Some(Self::Contact),
            2 => Some(Self::Reset),
            3 => Some(Self::Teleport),
            4 => Some(Self::Freeze),
            _ => None,
        }
    }
}

/// How the client's inputs of the last 32 ticks arrived. Counts saturate at 31.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct InputReport {
    pub early: u8,
    pub on_time: u8,
    pub late: u8,
    pub missing: u8,
    /// Mean earliness in ticks, 0..15.
    pub earliness: u8,
}

/// The own-car block.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OwnBlock {
    pub position: [f32; 3],
    pub orientation: [f32; 4],
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub own: OwnState,
    pub cause: CorrectionCause,
    pub report: InputReport,
}

impl OwnBlock {
    /// The block of a player's car `view` with its own state `own`.
    pub fn new(view: &EntityView, own: OwnState, cause: CorrectionCause, report: InputReport) -> Self {
        Self {
            position: view.position,
            orientation: view.orientation,
            linear_velocity: view.linear_velocity,
            angular_velocity: view.angular_velocity,
            own,
            cause,
            report,
        }
    }
}

impl GameSchema {
    /// Writes `block` as [`OWN_BLOCK_BITS`] bits.
    pub fn write_own(&self, block: &OwnBlock, w: &mut BitWriter<'_>) -> Result<(), SchemaError> {
        if w.remaining_bits() < OWN_BLOCK_BITS {
            return Err(SchemaError::Bits);
        }
        let mut put = |code: u32, bits: u32| w.write_bits(u64::from(code), bits).map_err(|_| SchemaError::Bits);
        for x in block.position {
            if !x.is_finite() {
                return Err(SchemaError::NotFinite);
            }
            if x.abs() > self.half_extent {
                return Err(SchemaError::OutOfBounds);
            }
            put(self.position.encode(x), POSITION_BITS)?;
        }
        put(pack_quat(block.orientation)?, 32)?;
        for x in block.linear_velocity {
            put(clamped(&quant::LINEAR_VELOCITY, x)?, quant::LINEAR_VELOCITY.bits())?;
        }
        for x in block.angular_velocity {
            put(clamped(&quant::ANGULAR_VELOCITY, x)?, quant::ANGULAR_VELOCITY.bits())?;
        }
        let own = &block.own;
        put(clamped(&quant::RPM, own.rpm)?, quant::RPM.bits())?;
        put(u32::from(own.gear.min(15)), 4)?;
        put(clamped(&quant::NITROUS, own.nitrous)?, quant::NITROUS.bits())?;
        put(clamped(&quant::BOOST, own.boost)?, quant::BOOST.bits())?;
        put(u32::from(own.flags), 8)?;
        put(block.cause as u32, 3)?;
        let report = &block.report;
        for count in [report.early, report.on_time, report.late, report.missing] {
            put(u32::from(count.min(31)), 5)?;
        }
        put(u32::from(report.earliness.min(15)), 4)?;
        put(0, (OWN_BLOCK_BITS - USED_BITS) as u32)
    }

    /// Reads a block written by [`GameSchema::write_own`]. A cause code above 4 or a nonzero reserved bit is
    /// [`SchemaError::Bits`].
    pub fn read_own(&self, r: &mut BitReader<'_>) -> Result<OwnBlock, SchemaError> {
        if r.remaining_bits() < OWN_BLOCK_BITS {
            return Err(SchemaError::Bits);
        }
        let mut take = |bits: u32| -> Result<u32, SchemaError> {
            let code = r.read_bits(bits).map_err(|_| SchemaError::Bits)?;
            u32::try_from(code).map_err(|_| SchemaError::Bits)
        };
        let mut block = OwnBlock::default();
        for slot in block.position.iter_mut() {
            *slot = value(&self.position, take(POSITION_BITS)?)?;
        }
        block.orientation = unpack_quat(take(32)?)?;
        for slot in block.linear_velocity.iter_mut() {
            *slot = value(&quant::LINEAR_VELOCITY, take(quant::LINEAR_VELOCITY.bits())?)?;
        }
        for slot in block.angular_velocity.iter_mut() {
            *slot = value(&quant::ANGULAR_VELOCITY, take(quant::ANGULAR_VELOCITY.bits())?)?;
        }
        block.own.rpm = value(&quant::RPM, take(quant::RPM.bits())?)?;
        block.own.gear = take(4)? as u8;
        block.own.nitrous = value(&quant::NITROUS, take(quant::NITROUS.bits())?)?;
        block.own.boost = value(&quant::BOOST, take(quant::BOOST.bits())?)?;
        block.own.flags = take(8)? as u8;
        block.cause = CorrectionCause::from_code(u64::from(take(3)?)).ok_or(SchemaError::Bits)?;
        block.report.early = take(5)? as u8;
        block.report.on_time = take(5)? as u8;
        block.report.late = take(5)? as u8;
        block.report.missing = take(5)? as u8;
        block.report.earliness = take(4)? as u8;
        if take((OWN_BLOCK_BITS - USED_BITS) as u32)? != 0 {
            return Err(SchemaError::Bits);
        }
        Ok(block)
    }
}
