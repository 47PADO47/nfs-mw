//! Typed readers for the parameter blocks of the operations that are understood. Offsets are in the block as the file holds it.
//! Each reader says in `docs/formats/chameleon-effects.md` how it was learned: **decompiled** (the executable's code was read),
//! **mapped** (the loader's copy from the file to the runtime object was read) or **inferred** (from the block's look alone).

use crate::ops::{
    EMITTER_CONE_PLUS, INITIALISE_2D_ROTATION, INITIALISE_AGE_AND_LIFETIME, INITIALISE_RADIUS,
    INITIALISE_VELOCITY_INHERITANCE, PARAMETRIC_POS_VEL_TO_WORLD_SPACE,
    PARAMETRIC_POS_VEL_TO_WORLD_SPACE_WITH_LOCAL_TRANSFORM, PARAMETRIC_SET_COLOUR_GRADIENT, PARAMETRIC_SET_SUB_UV,
    UPDATE_PARAMETRIC_RADIUS, UPDATE_POS_VEL,
};
use crate::{Emitter, f32_at, u32_at};

/// The lifetime of a particle: uniform in `[min, max]` seconds, times a scale that is `scale_floor` to 1 by the effect
/// parameter `channel` (decompiled).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lifetime {
    pub min: f32,
    pub max: f32,
    pub scale_floor: f32,
    pub channel: u8,
}

/// The starting radius of a particle: uniform in `[min, max]` (inferred from the block, the same pattern as the lifetime).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radius {
    pub min: f32,
    pub max: f32,
    pub scale_floor: f32,
}

/// The radius over the particle's life: `(lo * rise + base + hi * fall)` times the starting radius, where `t` is the age over
/// the lifetime, `lo = min(t, peak)` and `hi = max(t, peak)` (decompiled).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusCurve {
    pub peak: f32,
    pub base: f32,
    pub rise: f32,
    pub fall: f32,
    pub scale_floor: f32,
}

impl RadiusCurve {
    /// The factor on the starting radius at age fraction `t`.
    pub fn at(&self, t: f32) -> f32 {
        let (lo, hi) = (t.min(self.peak), t.max(self.peak));
        lo * self.rise + self.base + hi * self.fall
    }
}

/// How the particles move each step (decompiled, `UpdatePosVel`): position gains `velocity * dt`; the velocity is kept by
/// `(1 - drag)` **at every step, whatever its length** (so `drag` 0.75 leaves a quarter of it each step), pulled towards the
/// effect's wind by `drag * wind_blend`, gains `lift * dt` upwards and a random kick of up to `turbulence * dt` on each axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub lift: f32,
    pub drag: f32,
    pub wind_blend: f32,
    pub turbulence: f32,
}

/// The spawner (mapped, and decompiled where noted): particles per second `base_rate + rate_per_speed * speed of the emitter`,
/// within the time `window` of the effect, the camera `distance` range and the `speed_gate` range of the emitter; each
/// particle leaves in a random direction on the sphere with the speed `(velocity.x, .y, .z) * random(0..1)` and starts at
/// `shell` (a range of radii, times an effect parameter that is not decoded) from the emitter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    pub velocity: [f32; 3],
    pub window: [f32; 2],
    pub distance: [f32; 2],
    pub shell: [f32; 2],
    pub speed_gate: [f32; 2],
    pub base_rate: f32,
    pub rate_per_speed: f32,
    /// The spawner starts after a random delay of up to this many seconds (file +0x68, decompiled).
    pub delay: f32,
    /// Particles spawned at once when it starts (file +0x6C, decompiled: the spawn counter starts at this plus 0.9999).
    pub burst: u32,
    /// The camera distance range the spawn rate fades out over (file +0x20, mapped; the use is a distance fade).
    pub fade_distance: [f32; 2],
}

/// The colour and alpha of a particle over its life (decompiled): `colour = c0 + sum(w[i] * c[i + 1])` with
/// `w[i] = clamp(frac(t * colour_scale) * a[i] + b[i])`; the red, green and blue are multiplied by the fourth component (a
/// brightness that goes above 1); the alpha is a piecewise linear curve through five points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gradient {
    pub colours: [[f32; 4]; 5],
    pub a: [f32; 4],
    pub b: [f32; 4],
    pub colour_scale: f32,
    pub alpha_scale: f32,
    pub alpha_y: [f32; 5],
    pub alpha_x: [f32; 5],
    /// The effect's `ColourOverride` input replaces the colour (byte +0x9B, decompiled).
    pub colour_override: bool,
}

impl Gradient {
    /// The red, green, blue and brightness at age fraction `t`.
    pub fn colour_at(&self, t: f32) -> [f32; 4] {
        let x = t * self.colour_scale;
        let frac = x - x.floor();
        let w: [f32; 4] = std::array::from_fn(|i| (frac * self.a[i] + self.b[i]).clamp(0.0, 1.0));
        std::array::from_fn(|m| self.colours[0][m] + (0..4).map(|i| w[i] * self.colours[i + 1][m]).sum::<f32>())
    }

    /// The alpha at age fraction `t`: zero outside the five points.
    pub fn alpha_at(&self, t: f32) -> f32 {
        let x = t * self.alpha_scale;
        let x = x - x.floor();
        for i in 0..4 {
            let (x0, x1) = (self.alpha_x[i], self.alpha_x[i + 1]);
            if x0 <= x && x <= x1 && x1 > x0 {
                return self.alpha_y[i] + (self.alpha_y[i + 1] - self.alpha_y[i]) * (x - x0) / (x1 - x0);
            }
        }
        0.0
    }
}

/// The share of the emitter's velocity a particle takes at birth: uniform in `[min, max]` (inferred).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inherit {
    pub min: f32,
    pub max: f32,
}

/// The starting rotation: a range, and a spin number (inferred; the unit is not decoded).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation {
    pub min: f32,
    pub max: f32,
    pub spin: f32,
}

impl Emitter {
    fn block(&self, op: u8) -> Option<&[u8]> {
        self.task(op).map(|task| task.block.as_slice())
    }

    pub fn lifetime(&self) -> Option<Lifetime> {
        let b = self.block(INITIALISE_AGE_AND_LIFETIME)?;
        Some(Lifetime {
            min: f32_at(b, 0x0C)?,
            max: f32_at(b, 0x10)?,
            scale_floor: f32_at(b, 0x14)?,
            channel: *b.get(0x18)?,
        })
    }

    pub fn radius(&self) -> Option<Radius> {
        let b = self.block(INITIALISE_RADIUS)?;
        Some(Radius { min: f32_at(b, 0x08)?, max: f32_at(b, 0x0C)?, scale_floor: f32_at(b, 0x10)? })
    }

    pub fn radius_curve(&self) -> Option<RadiusCurve> {
        let b = self.block(UPDATE_PARAMETRIC_RADIUS)?;
        Some(RadiusCurve {
            peak: f32_at(b, 0x14)?,
            base: f32_at(b, 0x18)?,
            rise: f32_at(b, 0x1C)?,
            fall: f32_at(b, 0x20)?,
            scale_floor: f32_at(b, 0x24)?,
        })
    }

    pub fn motion(&self) -> Option<Motion> {
        let b = self.block(UPDATE_POS_VEL)?;
        Some(Motion {
            lift: f32_at(b, 0x0C)?,
            drag: f32_at(b, 0x10)?,
            wind_blend: f32_at(b, 0x14)?,
            turbulence: f32_at(b, 0x18)?,
        })
    }

    pub fn cone(&self) -> Option<Cone> {
        let b = self.block(EMITTER_CONE_PLUS)?;
        let pair = |at: usize| Some([f32_at(b, at)?, f32_at(b, at + 4)?]);
        Some(Cone {
            velocity: [f32_at(b, 0)?, f32_at(b, 4)?, f32_at(b, 8)?],
            window: pair(0x10)?,
            distance: pair(0x18)?,
            shell: pair(0x38)?,
            speed_gate: pair(0x40)?,
            base_rate: f32_at(b, 0x60)?,
            rate_per_speed: f32_at(b, 0x64)?,
            delay: f32_at(b, 0x68)?,
            burst: u32_at(b, 0x6C)?,
            fade_distance: pair(0x20)?,
        })
    }

    pub fn gradient(&self) -> Option<Gradient> {
        let b = self.block(PARAMETRIC_SET_COLOUR_GRADIENT)?;
        let vec4 = |at: usize| Some([f32_at(b, at)?, f32_at(b, at + 4)?, f32_at(b, at + 8)?, f32_at(b, at + 12)?]);
        let colours = [vec4(0x20)?, vec4(0x30)?, vec4(0x40)?, vec4(0x50)?, vec4(0x60)?];
        let curve = b.get(0x90..0x9A)?;
        let unit = |v: u8| f32::from(v) / 255.0;
        Some(Gradient {
            colours,
            a: vec4(0x70)?,
            b: vec4(0x80)?,
            colour_scale: f32_at(b, 0x9C)?,
            alpha_scale: f32_at(b, 0xA0)?,
            alpha_y: std::array::from_fn(|i| unit(curve[i])),
            alpha_x: std::array::from_fn(|i| unit(curve[5 + i])),
            colour_override: *b.get(0x9B)? != 0,
        })
    }

    pub fn inherit(&self) -> Option<Inherit> {
        let b = self.block(INITIALISE_VELOCITY_INHERITANCE)?;
        Some(Inherit { min: f32_at(b, 0)?, max: f32_at(b, 4)? })
    }

    pub fn rotation(&self) -> Option<Rotation> {
        let b = self.block(INITIALISE_2D_ROTATION)?;
        Some(Rotation { min: f32_at(b, 0)?, max: f32_at(b, 4)?, spin: f32_at(b, 0x14)? })
    }

    /// Whether the emitter's particles travel with the emitter: they are put into the world every frame from the emitter's
    /// current place (`ParametricPosVelToWorldSpace`, with or without the local transform), instead of once at birth.
    pub fn follows_emitter(&self) -> bool {
        self.task(PARAMETRIC_POS_VEL_TO_WORLD_SPACE).is_some()
            || self.task(PARAMETRIC_POS_VEL_TO_WORLD_SPACE_WITH_LOCAL_TRANSFORM).is_some()
    }

    /// Whether the emitter plays a flipbook (it sets a sub-picture and renders sub-pictures).
    pub fn flipbook(&self) -> bool {
        self.task(PARAMETRIC_SET_SUB_UV).is_some()
    }
}
