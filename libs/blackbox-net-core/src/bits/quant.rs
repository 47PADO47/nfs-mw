use std::f32::consts::FRAC_1_SQRT_2;
use std::f64::consts::{PI, TAU};

use super::{BitError, BitReader, BitWriter};

/// A uniform quantiser: `bits` bits spread evenly over `[min, max]`.
///
/// `bits` is `1..=32`. The caller must pass `min < max`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quant {
    min: f32,
    max: f32,
    bits: u32,
}

impl Quant {
    /// A quantiser over `[min, max]` with `bits` bits (`1..=32`).
    pub const fn new(min: f32, max: f32, bits: u32) -> Self {
        debug_assert!(matches!(bits, 1..=32));
        Self { min, max, bits }
    }

    /// The code for `x`: clamped to the range, rounded to the nearest step. NaN encodes as the value nearest 0.0.
    pub fn encode(&self, x: f32) -> u32 {
        let x = match x.is_nan() {
            true => 0.0,
            false => x,
        };
        let x = x.max(self.min).min(self.max);
        let span = f64::from(self.max) - f64::from(self.min);
        let top = self.top() as f64;
        let t = (f64::from(x) - f64::from(self.min)) / span;
        (t * top).round().clamp(0.0, top) as u32
    }

    /// The value for code `q`. A code above the top step is [`BitError::BadValue`].
    pub fn decode(&self, q: u32) -> Result<f32, BitError> {
        let top = self.top();
        if u64::from(q) > top {
            return Err(BitError::BadValue);
        }
        let span = f64::from(self.max) - f64::from(self.min);
        let x = f64::from(self.min) + f64::from(q) / top as f64 * span;
        Ok(x as f32)
    }

    /// The distance between two neighbouring codes.
    pub fn step(&self) -> f32 {
        ((f64::from(self.max) - f64::from(self.min)) / self.top() as f64) as f32
    }

    /// The number of bits per value.
    pub fn bits(&self) -> u32 {
        self.bits
    }

    /// Writes the code for `x` with [`Quant::bits`] bits.
    pub fn write(&self, w: &mut BitWriter<'_>, x: f32) -> Result<(), BitError> {
        w.write_bits(u64::from(self.encode(x)), self.bits)
    }

    /// Reads a code and decodes it.
    pub fn read(&self, r: &mut BitReader<'_>) -> Result<f32, BitError> {
        let q = r.read_bits(self.bits)?;
        let q = u32::try_from(q).map_err(|_| BitError::BadValue)?;
        self.decode(q)
    }

    /// The largest code, `2^bits - 1`.
    fn top(&self) -> u64 {
        (1u64 << self.bits.min(32)) - 1
    }
}

/// Encodes an angle in radians as `bits` bits over one full turn. The angle wraps into `[0, 2π)` first.
pub fn encode_angle(radians: f32, bits: u32) -> u32 {
    let steps = 1u64 << bits.min(32);
    let turn = TAU;
    let a = f64::from(radians).rem_euclid(turn);
    let q = (a / turn * steps as f64).round() as u64;
    (q % steps) as u32
}

/// Decodes an angle written by [`encode_angle`], returned in `[-π, π)`.
pub fn decode_angle(q: u32, bits: u32) -> f32 {
    let steps = 1u64 << bits.min(32);
    let turn = TAU;
    let index = u64::from(q) % steps;
    let mut a = index as f64 / steps as f64 * turn;
    if a >= PI {
        a -= turn;
    }
    a as f32
}

/// Writes a unit quaternion `[x, y, z, w]` in smallest-three form.
///
/// The index of the largest component takes 2 bits. The other three follow, sign flipped so the largest is positive,
/// each quantised over `[-1/√2, 1/√2]` with `bits` bits (`1..=32`). Nothing is written when the room is short.
pub fn write_quat(w: &mut BitWriter<'_>, q: [f32; 4], bits: u32) -> Result<(), BitError> {
    if !matches!(bits, 1..=32) {
        return Err(BitError::BadValue);
    }
    if 2 + 3 * bits as usize > w.remaining_bits() {
        return Err(BitError::Overflow);
    }
    let largest = largest_index(&q);
    let sign = match q[largest] < 0.0 {
        true => -1.0,
        false => 1.0,
    };
    w.write_bits(largest as u64, 2)?;
    let quant = Quant::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2, bits);
    for (i, c) in q.iter().enumerate() {
        if i == largest {
            continue;
        }
        quant.write(w, sign * *c)?;
    }
    Ok(())
}

/// Reads a quaternion written by [`write_quat`]. The dropped component is rebuilt from the other three and the
/// result is normalised.
pub fn read_quat(r: &mut BitReader<'_>, bits: u32) -> Result<[f32; 4], BitError> {
    if !matches!(bits, 1..=32) {
        return Err(BitError::BadValue);
    }
    let largest = usize::try_from(r.read_bits(2)?).map_err(|_| BitError::BadValue)?;
    let quant = Quant::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2, bits);
    let mut q = [0.0f32; 4];
    let mut sum_sq = 0.0f64;
    for (i, slot) in q.iter_mut().enumerate() {
        if i == largest {
            continue;
        }
        let c = quant.read(r)?;
        *slot = c;
        sum_sq += f64::from(c) * f64::from(c);
    }
    let rest = (1.0 - sum_sq).max(0.0).sqrt() as f32;
    let slot = q.get_mut(largest).ok_or(BitError::BadValue)?;
    *slot = rest;
    let norm = q.iter().map(|c| f64::from(*c) * f64::from(*c)).sum::<f64>().sqrt();
    if norm > 0.0 {
        for c in q.iter_mut() {
            *c = (f64::from(*c) / norm) as f32;
        }
    }
    Ok(q)
}

/// Index of the component with the largest magnitude. Ties keep the lowest index.
fn largest_index(q: &[f32; 4]) -> usize {
    let mut best = 0;
    for (i, c) in q.iter().enumerate() {
        if c.abs() > q[best].abs() {
            best = i;
        }
    }
    best
}
