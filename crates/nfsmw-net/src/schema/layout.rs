//! The field layout of every entity kind: widths, difference coding and the index of each field.
//!
//! Positions are difference-coded (a moving car changes a few steps per snapshot); everything else is raw. A field
//! list only grows at its end, and any change to it changes the schema id.

use blackbox_net_core::replicate::FieldDesc;

use super::quant::{ORIENTATION_BITS, POSITION_BITS};

const fn raw(bits: u8) -> FieldDesc {
    FieldDesc { bits, diff: false }
}

const fn diff(bits: u8) -> FieldDesc {
    FieldDesc { bits, diff: true }
}

const POS: FieldDesc = diff(POSITION_BITS as u8);
const STATIC_POS: FieldDesc = raw(POSITION_BITS as u8);
const ORIENT: FieldDesc = raw(ORIENTATION_BITS as u8);
const TYPE_ID: FieldDesc = raw(16);
const OWNER: FieldDesc = raw(OWNER_BITS as u8);

/// Bits of the owner field: 0 for none, else the player id plus one.
pub const OWNER_BITS: u32 = 6;

/// Index of each field of the full player car.
pub mod car {
    pub const POS: usize = 0;
    pub const ORIENT: usize = 3;
    pub const VEL: usize = 4;
    pub const ANG: usize = 7;
    pub const WHEEL_COMPRESSION: usize = 10;
    pub const WHEEL_GROUND: usize = 11;
    pub const WHEEL_STEER: usize = 12;
    pub const SKID: usize = 13;
    pub const RPM: usize = 14;
    pub const GEAR: usize = 15;
    pub const FLAGS: usize = 16;
    pub const TYPE_ID: usize = 17;
    pub const OWNER: usize = 18;
    pub const INPUTS: usize = 19;
    /// Bits of the car's flags field.
    pub const FLAG_BITS: u32 = 16;
}

/// The full player car: pose, motion, wheels, engine, flags, type, owner and the inputs for the remote look.
pub const PLAYER_CAR: [FieldDesc; 20] = [
    POS,
    POS,
    POS,
    ORIENT,
    raw(14),
    raw(14),
    raw(14),
    raw(10),
    raw(10),
    raw(10),
    raw(24),
    raw(4),
    raw(8),
    raw(8),
    raw(10),
    raw(4),
    raw(car::FLAG_BITS as u8),
    TYPE_ID,
    OWNER,
    raw(20),
];

/// Index of each field of the yaw-and-speed kinds (reduced player car, traffic, police, roadblock).
pub mod flat {
    pub const YAW: usize = 3;
    pub const SPEED: usize = 4;
}

/// Bits of the yaw of a reduced player car, a traffic car and a roadblock.
pub const COARSE_YAW_BITS: u32 = 8;
/// Bits of the yaw of a police car.
pub const POLICE_YAW_BITS: u32 = 10;

/// The reduced player car (the "always" tier for minimap and names): position, yaw, speed, flags, owner.
pub const PLAYER_CAR_REDUCED: [FieldDesc; 7] = [POS, POS, POS, raw(8), raw(8), raw(8), OWNER];
/// Flag bits of a reduced player car.
pub const REDUCED_FLAG_BITS: u32 = 8;

/// A traffic car: position, yaw, speed, flags (brake lights, signal), type.
pub const TRAFFIC_CAR: [FieldDesc; 7] = [POS, POS, POS, raw(8), raw(9), raw(TRAFFIC_FLAG_BITS as u8), TYPE_ID];
/// Flag bits of a traffic car.
pub const TRAFFIC_FLAG_BITS: u32 = 8;

/// A police car: as traffic with a finer yaw and speed, and 16 flag bits (siren, lights, pursuit goal, target).
pub const POLICE_CAR: [FieldDesc; 7] = [POS, POS, POS, raw(10), raw(10), raw(POLICE_FLAG_BITS as u8), TYPE_ID];
/// Flag bits of a police car.
pub const POLICE_FLAG_BITS: u32 = 16;

/// Index of each field of the kinds with a full orientation (helicopter, moving prop).
pub mod posed {
    pub const ORIENT: usize = 3;
    pub const FLAGS: usize = 4;
    pub const TYPE_ID: usize = 5;
}

/// A helicopter: position, orientation, 24 flag bits (rotor state, spotlight target), type.
pub const HELICOPTER: [FieldDesc; 6] = [POS, POS, POS, ORIENT, raw(HELICOPTER_FLAG_BITS as u8), TYPE_ID];
/// Flag bits of a helicopter.
pub const HELICOPTER_FLAG_BITS: u32 = 24;

/// A roadblock or spike strip: a still position, yaw, state flags, type.
pub const ROADBLOCK: [FieldDesc; 6] =
    [STATIC_POS, STATIC_POS, STATIC_POS, raw(8), raw(ROADBLOCK_FLAG_BITS as u8), TYPE_ID];
/// Flag bits of a roadblock (the state in the low two).
pub const ROADBLOCK_FLAG_BITS: u32 = 8;

/// A moving prop: position, orientation, flags (broken), type.
pub const MOVING_PROP: [FieldDesc; 6] = [POS, POS, POS, ORIENT, raw(PROP_FLAG_BITS as u8), TYPE_ID];
/// Flag bits of a moving prop.
pub const PROP_FLAG_BITS: u32 = 8;
