//! The game's entity schema for `blackbox_net_core::replicate` (protocol.md section 6.3, bandwidth.md section 1.1,
//! ADR 0009 section 4), and the conversion of [`nfsmw_sim::EntityView`] and [`nfsmw_sim::OwnState`] to and from it.
//!
//! - [`WireKind`]: the seven kinds of protocol.md 6.3. The replicate [`KindId`] of a kind is its wire number minus one,
//!   because [`GameSchema::new`] registers them in wire order.
//! - [`GameSchema`]: the registered [`Schema`], the position range (`world_bounds`) and [`GameSchema::schema_id`], the
//!   fingerprint `JoinAccepted` carries. Field widths and ranges: `layout.rs` and `quant.rs`.
//! - [`GameSchema::encode`] and [`GameSchema::encode_reduced`] quantise a view into a [`Record`] (no allocation), ready
//!   for `Frame::push`; [`GameSchema::decode`] turns a record back into a [`Decoded`] view. Kinds without a full
//!   orientation carry yaw and forward speed only, so their decoded pitch, roll, vertical and angular velocity are zero.
//! - [`OwnBlock`]: the receiving player's own-car block (256 bits) with [`GameSchema::write_own`] and
//!   [`GameSchema::read_own`].
//!
//! Encoding refuses a non-finite number, a position outside the bounds and a zero quaternion; velocities and other
//! ranges are clamped. Flag bits above a kind's flag width are not replicated. Decoding refuses a record of an unknown
//! kind or of the wrong length.

mod convert;
mod layout;
mod own;
mod quant;

#[cfg(test)]
mod tests;

pub use convert::{CarDetail, Decoded, Record, Tier};
pub use layout::{
    COARSE_YAW_BITS, HELICOPTER_FLAG_BITS, OWNER_BITS, POLICE_FLAG_BITS, POLICE_YAW_BITS, PROP_FLAG_BITS,
    REDUCED_FLAG_BITS, ROADBLOCK_FLAG_BITS, TRAFFIC_FLAG_BITS, car,
};
pub use own::{CorrectionCause, InputReport, OWN_BLOCK_BITS, OwnBlock};
pub use quant::{
    ANGULAR_VELOCITY, BOOST, LINEAR_VELOCITY, NITROUS, POLICE_SPEED, POSITION_BITS, REDUCED_SPEED, RPM, TRAFFIC_SPEED,
    pack_quat, quat_of_yaw, unpack_quat, yaw_of,
};

use blackbox_net_core::bits::Quant;
use blackbox_net_core::replicate::{FieldDesc, KindId, Schema};
use nfsmw_sim::EntityKind;

/// Why a view could not be encoded or a record decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SchemaError {
    #[error("a value is not finite")]
    NotFinite,
    #[error("a position is outside the world bounds")]
    OutOfBounds,
    #[error("a quaternion is zero")]
    BadQuaternion,
    #[error("a player id does not fit the owner field")]
    BadOwner,
    #[error("a record has an unknown kind or the wrong number of fields")]
    BadRecord,
    #[error("a code is outside its field")]
    BadValue,
    #[error("the world bounds are not a positive finite size")]
    BadBounds,
    #[error("the bit stream is too short or has nonzero reserved bits")]
    Bits,
}

/// An entity kind on the wire (protocol.md section 6.3). The number is the wire kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WireKind {
    PlayerCar = 1,
    /// A player car's "always" tier: position, yaw, speed, role and state flags.
    PlayerCarReduced = 2,
    TrafficCar = 3,
    PoliceCar = 4,
    Helicopter = 5,
    Roadblock = 6,
    MovingProp = 7,
}

impl WireKind {
    /// Every kind, in registration order.
    pub const ALL: [Self; 7] = [
        Self::PlayerCar,
        Self::PlayerCarReduced,
        Self::TrafficCar,
        Self::PoliceCar,
        Self::Helicopter,
        Self::Roadblock,
        Self::MovingProp,
    ];

    /// The replicate kind id: the wire number minus one.
    pub fn kind_id(self) -> KindId {
        KindId(self as u8 - 1)
    }

    /// The kind of replicate kind id `kind`, or `None` for an id no kind has.
    pub fn from_kind_id(kind: KindId) -> Option<Self> {
        Self::ALL.get(usize::from(kind.0)).copied()
    }

    /// The full-tier wire kind of a sim entity kind.
    pub fn of(kind: EntityKind) -> Self {
        match kind {
            EntityKind::PlayerCar => Self::PlayerCar,
            EntityKind::TrafficCar => Self::TrafficCar,
            EntityKind::PoliceCar => Self::PoliceCar,
            EntityKind::Helicopter => Self::Helicopter,
            EntityKind::Roadblock => Self::Roadblock,
            EntityKind::MovingProp => Self::MovingProp,
        }
    }

    /// The sim entity kind; both player car tiers are a player car.
    pub fn entity_kind(self) -> EntityKind {
        match self {
            Self::PlayerCar | Self::PlayerCarReduced => EntityKind::PlayerCar,
            Self::TrafficCar => EntityKind::TrafficCar,
            Self::PoliceCar => EntityKind::PoliceCar,
            Self::Helicopter => EntityKind::Helicopter,
            Self::Roadblock => EntityKind::Roadblock,
            Self::MovingProp => EntityKind::MovingProp,
        }
    }

    /// The kind's field layout.
    pub fn fields(self) -> &'static [FieldDesc] {
        match self {
            Self::PlayerCar => &layout::PLAYER_CAR,
            Self::PlayerCarReduced => &layout::PLAYER_CAR_REDUCED,
            Self::TrafficCar => &layout::TRAFFIC_CAR,
            Self::PoliceCar => &layout::POLICE_CAR,
            Self::Helicopter => &layout::HELICOPTER,
            Self::Roadblock => &layout::ROADBLOCK,
            Self::MovingProp => &layout::MOVING_PROP,
        }
    }
}

/// The game's registered schema and position range. Built the same way on the server and every client.
#[derive(Debug, Clone)]
pub struct GameSchema {
    schema: Schema,
    position: Quant,
    half_extent: f32,
}

impl Default for GameSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSchema {
    /// The default world bounds: ±8,192 m on every axis (an assumed city extent, protocol.md open question 2).
    pub const DEFAULT_HALF_EXTENT: f32 = 8192.0;
    /// The largest half extent accepted: ±1,000 km.
    pub const MAX_HALF_EXTENT: f32 = 1.0e6;

    /// The schema with the default bounds.
    pub fn new() -> Self {
        Self::build(Self::DEFAULT_HALF_EXTENT)
    }

    /// The schema with positions over ±`half_extent` metres (`world_bounds` of `JoinAccepted`).
    pub fn with_half_extent(half_extent: f32) -> Result<Self, SchemaError> {
        if !half_extent.is_finite() || half_extent <= 0.0 || half_extent > Self::MAX_HALF_EXTENT {
            return Err(SchemaError::BadBounds);
        }
        Ok(Self::build(half_extent))
    }

    fn build(half_extent: f32) -> Self {
        let mut schema = Schema::new();
        for kind in WireKind::ALL {
            // Seven kinds of at most twenty fields of 1..=32 bits always register.
            let registered = schema.add_kind(kind.fields());
            debug_assert_eq!(registered.ok(), Some(kind.kind_id()));
        }
        let position = Quant::new(-half_extent, half_extent, POSITION_BITS);
        Self { schema, position, half_extent }
    }

    /// The registered replicate schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Half the side of the position range, in metres.
    pub fn half_extent(&self) -> f32 {
        self.half_extent
    }

    /// The distance between two position codes, in metres.
    pub fn position_step(&self) -> f32 {
        self.position.step()
    }

    /// The fingerprint of the field layout and every range: FNV-1a 64 over the replicate schema id, the half extent and
    /// the fixed ranges, as little-endian bytes.
    pub fn schema_id(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |bytes: &[u8]| {
            for byte in bytes {
                hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        feed(&self.schema.schema_id().to_le_bytes());
        feed(&self.half_extent.to_bits().to_le_bytes());
        for range in quant::RANGES {
            feed(&range.bits().to_le_bytes());
            feed(&range.step().to_bits().to_le_bytes());
        }
        hash
    }
}
