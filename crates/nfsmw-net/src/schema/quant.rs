//! The quantisers of the game's fields (bandwidth.md section 1.1) and the packing of several small values into one
//! replicated field. World axes: x forward, y left, z up; yaw turns about z.

use blackbox_net_core::bits::{BitReader, BitWriter, Quant, decode_angle, encode_angle, read_quat, write_quat};

use super::SchemaError;

/// Bits per position axis: ±half extent in about 1.6 cm steps at the default 8,192 m.
pub const POSITION_BITS: u32 = 20;
/// Bits of a smallest-three orientation: a 2-bit index and three 10-bit components.
pub const ORIENTATION_BITS: u32 = 32;
/// Bits per smallest-three component.
pub const QUAT_COMPONENT_BITS: u32 = 10;
/// Linear velocity per axis: ±400 m/s in about 5 cm/s steps. Faster values are clamped.
pub const LINEAR_VELOCITY: Quant = Quant::new(-400.0, 400.0, 14);
/// Angular velocity per axis: ±16 rad/s in about 0.03 rad/s steps. Faster values are clamped.
pub const ANGULAR_VELOCITY: Quant = Quant::new(-16.0, 16.0, 10);
/// Engine speed: 0 to 10,230 rpm in 10 rpm steps.
pub const RPM: Quant = Quant::new(0.0, 10_230.0, 10);
/// Suspension compression per wheel, 0..1.
pub const WHEEL_COMPRESSION: Quant = Quant::new(0.0, 1.0, 6);
/// Front-wheel steer, -1..1 of full lock.
pub const WHEEL_STEER: Quant = Quant::new(-1.0, 1.0, 8);
/// Throttle and brake of the remote look, 0..1.
pub const PEDAL_LOOK: Quant = Quant::new(0.0, 1.0, 6);
/// Steering of the remote look, -1..1.
pub const STEER_LOOK: Quant = Quant::new(-1.0, 1.0, 8);
/// Forward speed of a traffic car: -32 to 96 m/s in 0.25 m/s steps.
pub const TRAFFIC_SPEED: Quant = Quant::new(-32.0, 96.0, 9);
/// Forward speed of a police car: -32 to 96 m/s in 0.125 m/s steps.
pub const POLICE_SPEED: Quant = Quant::new(-32.0, 96.0, 10);
/// Forward speed of a reduced player record: -32 to 96 m/s in 0.5 m/s steps.
pub const REDUCED_SPEED: Quant = Quant::new(-32.0, 96.0, 8);
/// Nitrous left in the own block, 0..1.
pub const NITROUS: Quant = Quant::new(0.0, 1.0, 7);
/// Speedbreaker meter in the own block, 0..1.
pub const BOOST: Quant = Quant::new(0.0, 1.0, 6);

/// Every fixed range, in a stable order, for the schema fingerprint.
pub(super) const RANGES: [Quant; 13] = [
    LINEAR_VELOCITY,
    ANGULAR_VELOCITY,
    RPM,
    WHEEL_COMPRESSION,
    WHEEL_STEER,
    PEDAL_LOOK,
    STEER_LOOK,
    TRAFFIC_SPEED,
    POLICE_SPEED,
    REDUCED_SPEED,
    NITROUS,
    BOOST,
    Quant::new(-std::f32::consts::FRAC_1_SQRT_2, std::f32::consts::FRAC_1_SQRT_2, QUAT_COMPONENT_BITS),
];

/// The code of a finite `x`, clamped into `quant`'s range.
pub fn clamped(quant: &Quant, x: f32) -> Result<u32, SchemaError> {
    if !x.is_finite() {
        return Err(SchemaError::NotFinite);
    }
    Ok(quant.encode(x))
}

/// The value of `code`.
pub fn value(quant: &Quant, code: u32) -> Result<f32, SchemaError> {
    quant.decode(code).map_err(|_| SchemaError::BadValue)
}

/// A unit quaternion `[x, y, z, w]` in smallest-three form, as one 32-bit field. It is normalised first; a zero or
/// non-finite quaternion is refused.
pub fn pack_quat(q: [f32; 4]) -> Result<u32, SchemaError> {
    if q.iter().any(|c| !c.is_finite()) {
        return Err(SchemaError::NotFinite);
    }
    let norm = q.iter().map(|c| f64::from(*c) * f64::from(*c)).sum::<f64>().sqrt();
    if norm < 1e-6 {
        return Err(SchemaError::BadQuaternion);
    }
    let unit = q.map(|c| (f64::from(c) / norm) as f32);
    let mut buf = [0u8; 4];
    write_quat(&mut BitWriter::new(&mut buf), unit, QUAT_COMPONENT_BITS).map_err(|_| SchemaError::BadQuaternion)?;
    Ok(u32::from_be_bytes(buf))
}

/// The quaternion of a field written by [`pack_quat`], normalised.
pub fn unpack_quat(code: u32) -> Result<[f32; 4], SchemaError> {
    let buf = code.to_be_bytes();
    read_quat(&mut BitReader::new(&buf), QUAT_COMPONENT_BITS).map_err(|_| SchemaError::BadQuaternion)
}

/// The yaw (heading about z, radians) of a unit quaternion.
pub fn yaw_of(q: [f32; 4]) -> f32 {
    let [x, y, z, w] = q.map(f64::from);
    (2.0 * (w * z + x * y)).atan2(1.0 - 2.0 * (y * y + z * z)) as f32
}

/// The unit quaternion of a pure yaw.
pub fn quat_of_yaw(yaw: f32) -> [f32; 4] {
    let half = f64::from(yaw) * 0.5;
    [0.0, 0.0, half.sin() as f32, half.cos() as f32]
}

/// The yaw of `q` as `bits` bits over a full turn.
pub fn pack_yaw(q: [f32; 4], bits: u32) -> Result<u32, SchemaError> {
    if q.iter().any(|c| !c.is_finite()) {
        return Err(SchemaError::NotFinite);
    }
    Ok(encode_angle(yaw_of(q), bits))
}

/// The yaw of a field written by [`pack_yaw`].
pub fn unpack_yaw(code: u32, bits: u32) -> f32 {
    decode_angle(code, bits)
}

/// The speed along the heading `yaw` of `velocity`.
pub fn forward_speed(velocity: [f32; 3], yaw: f32) -> f32 {
    velocity[0] * yaw.cos() + velocity[1] * yaw.sin()
}

/// The horizontal velocity of `speed` along `yaw`.
pub fn velocity_of(speed: f32, yaw: f32) -> [f32; 3] {
    [speed * yaw.cos(), speed * yaw.sin(), 0.0]
}

/// `codes`, each `bits` wide, packed with the first in the highest bits.
pub fn pack(codes: &[u32], bits: u32) -> u32 {
    codes.iter().fold(0, |acc, code| (acc << bits) | (code & ((1 << bits) - 1)))
}

/// The `N` codes of `bits` each that [`pack`] wrote into `field`.
pub fn unpack<const N: usize>(field: u32, bits: u32) -> [u32; N] {
    let mut out = [0u32; N];
    for (index, slot) in out.iter_mut().enumerate() {
        let shift = bits * (N - 1 - index) as u32;
        *slot = (field >> shift) & ((1 << bits) - 1);
    }
    out
}
