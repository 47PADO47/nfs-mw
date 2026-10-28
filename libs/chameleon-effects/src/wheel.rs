//! The wheel spawner (`EmitterWheel`, `EmitterWheelContacts`) and the wheel bindings (`UpdateBindings_Wheel`): the operations
//! of the effects that a surface names for a tyre (smoke, dust, sparks). Where the cone spawner of `decode` throws particles
//! from a point, these spawn them around a wheel's contact patch and scale the rate by the wheel's own inputs.
//!
//! How it was learned: the execute function of `EmitterWheel` was decompiled (**decompiled**); the runtime copy of its block is
//! laid out like the file's block with four floats added at the front and one accumulator after the rate (counted from the
//! nine slot references that sit in both), which gives the offsets below (**mapped**). The wheel input that scales the rate
//! (`w` below) is a field of the wheel's binding record that is not decoded: a tyre that smokes more has a larger `w`.

use crate::{Emitter, f32_at};

/// The spawner of a wheel effect. `rate` is per second at `w = 1`.
///
/// Particles born per second for one wheel (decompiled): `w * (rate + rate_per_speed * speed)`, times the effect parameter
/// `P[params[0]]`, times a fade over the distance to the camera (full below `fade_distance[0]`, gone beyond `[1]`); `speed` is the
/// larger of the car's speed and the speed of the tyre's contact patch (so a spinning tyre on the spot still smokes). A particle is
/// born on the tyre: at an angle drawn from `angle` degrees around the axle, at `radius_scale` times the wheel's radius and across
/// the tread within `width_scale` times its half width, and leaves at a speed drawn from `speed` metres a second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelSpawner {
    /// The squared distance range the wheel must be in from a reference position for the emitter to spawn (file +0x00).
    pub gate: [f32; 2],
    /// The camera distance the rate fades out over, near and far (file +0x08, +0x0C; the fade shape is not decoded).
    pub fade_distance: [f32; 2],
    /// The speed a new particle leaves at, a range in metres a second (file +0x10, +0x14).
    pub speed: [f32; 2],
    /// The angle around the axle, degrees (file +0x28, +0x2C).
    pub angle: [f32; 2],
    /// Where on the tyre a particle starts, as a share of the wheel's radius (file +0x30, +0x34) and of half its tread width
    /// (+0x38, +0x3C: minus one to one is the whole width).
    pub radius_scale: [f32; 2],
    pub width_scale: [f32; 2],
    /// The height above the road a particle starts at, a range in metres (file +0x40, +0x44).
    pub height: [f32; 2],
    /// The rate a second at `w = 1` (file +0x74) and for each m/s of speed (+0x70; when it is 0 the value at +0x6C is used).
    pub rate: f32,
    pub rate_per_speed: f32,
    /// The index into the effect's parameter array of the rate scale, the speed scale and the height scale, and a quality gate
    /// (file bytes +0x78..+0x7B).
    pub params: [u8; 4],
}

/// The size of the block of `UpdatePosVelWithBouncePlane` in the file.
const BOUNCE_BLOCK: usize = 0x40;
/// The size of the block of `EmitterWheel` in the file.
const WHEEL_BLOCK: usize = 0x7C;

impl Emitter {
    /// The share of its velocity a particle that bounces on the road loses at every step (`UpdatePosVelWithBouncePlane`, file
    /// +0x34; **inferred**: of the four numbers after the plane's vector it is the one that, read as the per-step drag of
    /// `UpdatePosVel`, leaves the smoke clouds where the tyre made them). `None` for an emitter without the operation.
    pub fn bounce_drag(&self) -> Option<f32> {
        let b = self.task(crate::ops::UPDATE_POS_VEL_WITH_BOUNCE_PLANE)?.block.as_slice();
        if b.len() < BOUNCE_BLOCK {
            return None;
        }
        f32_at(b, 0x34)
    }

    /// The wheel spawner of the emitter, if it has one (`EmitterWheel`).
    pub fn wheel(&self) -> Option<WheelSpawner> {
        let b = self.task(crate::ops::EMITTER_WHEEL)?.block.as_slice();
        if b.len() < WHEEL_BLOCK {
            return None;
        }
        let pair = |at: usize| Some([f32_at(b, at)?, f32_at(b, at + 4)?]);
        let at_speed = f32_at(b, 0x70)?;
        Some(WheelSpawner {
            gate: pair(0x00)?,
            fade_distance: pair(0x08)?,
            speed: pair(0x10)?,
            angle: pair(0x28)?,
            radius_scale: pair(0x30)?,
            width_scale: pair(0x38)?,
            height: pair(0x40)?,
            rate: f32_at(b, 0x74)?,
            rate_per_speed: if at_speed > 0.0 { at_speed } else { f32_at(b, 0x6C)? },
            params: [*b.get(0x78)?, *b.get(0x79)?, *b.get(0x7A)?, *b.get(0x7B)?],
        })
    }
}

#[cfg(test)]
#[path = "wheel_tests.rs"]
mod tests;
