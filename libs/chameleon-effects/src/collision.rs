//! The operations of the collision effects: the spawner that throws sparks and dust from the surface of the car that was hit
//! (`EmitterCollisionMesh`), the bounce of a particle off a plane (`UpdatePosVelWithBouncePlane`), the sprite that is
//! stretched along its velocity (`RenderSpriteVelocity`, the spark streak) and the bindings that turn the effect's inputs into
//! the parameter array `P` (`UpdateBindings`).
//!
//! How each was learned is tagged as in `docs/formats/chameleon-effects.md`: **decompiled** (the executable's execute function
//! was read), **mapped** (the loader's copy from the file to the runtime object was read) or **inferred**. The file offsets are
//! those of the parameter block as the file holds it. The format page, "Collision effects", has the numbers of the installed
//! effects and what is not decoded.

use crate::{Effect, Emitter, Error, f32_at, u16_at, u32_at};

pub const EMITTER_COLLISION_MESH: u8 = 1;
pub const INITIALISE_BOUNCE_PLANE: u8 = 19;
pub const RENDER_SPRITE_MOTION_BLUR: u8 = 37;
pub const RENDER_SPRITE_VELOCITY: u8 = 41;
pub const UPDATE_BINDINGS: u8 = 51;
pub const UPDATE_PARAMETRIC_FADE_OVER_DISTANCE: u8 = 56;
pub const UPDATE_POS_VEL_WITH_BOUNCE_PLANE: u8 = 62;

/// The spawner of a collision effect (decompiled for the use of the numbers, mapped for the offsets). It does not spawn at a
/// point: it walks the triangles of the collision mesh of the object that was hit, keeps those that are within `radius` of the
/// effect's position (times `P[params[0]]`), less than `height` above the contact plane and facing up enough, and gives each of
/// them `area * (burst * P[params[2]] + rate * dt * P[params[1]]) * (fade over the camera distance)` particles, so the numbers
/// are **per square metre of the surface**. A particle starts at a random point of the triangle and leaves along a random
/// direction of the hemisphere above the triangle, at `speed.0 .. speed.1` metres a second times `P[params[3]]`.
///
/// `burst` is spent on the first update only; `rate` goes on for the `window` of the effect's time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionMesh {
    /// The speed a particle leaves at, a range in metres a second (file +0x00, +0x04).
    pub speed: [f32; 2],
    /// Two ranges in degrees that shape the direction (file +0x08..+0x14), scaled by `P[params[4]]`. Zero ranges in every
    /// installed effect, so the direction is the plain hemisphere.
    pub angle: [[f32; 2]; 2],
    /// The effect's time, in seconds, in which `rate` spawns (file +0x18, +0x1C).
    pub window: [f32; 2],
    /// The camera distance range the count fades out over (file +0x20, +0x24).
    pub fade_distance: [f32; 2],
    /// How far from the effect's position, in metres, a triangle may be (file +0x48).
    pub radius: f32,
    /// How far above the contact plane, in metres, a triangle may be (file +0x4C).
    pub height: f32,
    /// The triangle's normal must satisfy `(dot(normal, up) + 1) / 2 > facing` (file +0x50).
    pub facing: f32,
    /// Particles per square metre and per second (file +0x54) and at once on the first update (+0x58).
    pub rate: u32,
    pub burst: u32,
    /// The indices into `P` of the radius scale, the rate scale, the burst scale, the speed scale and the angle scale, and the
    /// quality level that must be below the game's detail setting (file bytes +0x5C..+0x61).
    pub params: [u8; 5],
    pub quality: u8,
}

/// The bounce and the motion of a particle that falls to a plane (decompiled for the use, mapped for the offsets). Each step
/// the velocity is kept by `1 - drag`, gains `gravity * dt` upwards and a random kick of up to `turbulence * dt`; a particle
/// that falls through the contact plane is put back on it and its normal speed is reflected and multiplied by a random number in
/// `restitution`, its tangent speed multiplied by `1 - random(friction)` (and a bounce too weak to lift the particle for one
/// step is left as a slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounce {
    /// Share of the normal speed kept, a range drawn at random for every particle (file +0x1C, +0x20).
    pub restitution: [f32; 2],
    /// Share of the tangent speed lost on a bounce, a range (file +0x24, +0x28).
    pub friction: [f32; 2],
    /// Vertical acceleration in metres a second squared (file +0x2C).
    pub gravity: f32,
    /// Share of the velocity lost every step (file +0x30). Whether a step is a frame or a fixed time is not decoded.
    pub drag: f32,
    /// Wind blend and turbulence (file +0x34, +0x38).
    pub wind_blend: f32,
    pub turbulence: f32,
}

/// The sprite that is stretched along the particle's velocity (`RenderSpriteVelocity`; decompiled, mapped). The segment drawn
/// has the length `|velocity| * length_per_speed * C * P[scale_param]` (`C` is a constant of the executable that is set at
/// start-up and not in its file; taken as 1, **not decoded**), clamped to `length` metres, and is as wide as the particle's
/// radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityStreak {
    /// The shortest and longest segment, in metres (file +0x30, +0x34).
    pub length: [f32; 2],
    /// The segment per unit of speed (file +0x48): 0.125 in every installed effect.
    pub length_per_speed: f32,
    /// The brightness floor (file +0x40) and the opacity scale (+0x44).
    pub floor: f32,
    pub opacity: f32,
    /// The index into `P` that scales the length (file byte +0x4C).
    pub scale_param: u8,
}

/// Where a binding takes its value from (decompiled; the numbers are the selector byte of the file at +0x60 + n).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// `scale * time` of the effect (0, 1 wrapped to a fraction).
    Time,
    /// `scale * |Velocity|`, the speed of the emitter (3).
    Speed,
    /// `scale * distance to the camera` (4).
    CameraDistance,
    /// A constant, `scale` (9).
    Constant,
    /// The scalar input of the effect at slot 0: `Intensity` in the collision effects (10).
    InputA,
    /// `input B / scale` (17; `ImpactClosingVelocity` in the collision effects).
    InputBOver,
    /// `input C / scale` (18; `MovementSinceLastFrame`).
    InputCOver,
    /// Another selector of the executable's switch whose use is not read.
    Other(u8),
}

impl Source {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0 | 1 => Source::Time,
            3 => Source::Speed,
            4 => Source::CameraDistance,
            9 => Source::Constant,
            10 => Source::InputA,
            0x11 => Source::InputBOver,
            0x12 => Source::InputCOver,
            other => Source::Other(other),
        }
    }
}

/// One of the four bindings of `UpdateBindings` (decompiled): `P[n + 1] = curve(source(scale)) + bias`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Binding {
    pub source: Source,
    pub scale: f32,
    pub bias: f32,
    /// The response curve: five points `(x, y)`, straight lines between, held flat before the first and after the last.
    /// The file keeps each coordinate as a byte (`byte / 255`; a range multiplier of the executable that is set at start-up
    /// is taken as 1, **not decoded**).
    pub curve: [(f32, f32); 5],
}

impl Binding {
    /// The value of `P` for a source value `x` that has already been multiplied or divided by `scale`.
    pub fn response(&self, x: f32) -> f32 {
        let first = self.curve[0];
        let last = self.curve[4];
        if x <= first.0 {
            return first.1 + self.bias;
        }
        if x >= last.0 {
            return last.1 + self.bias;
        }
        for pair in self.curve.windows(2) {
            let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
            if x <= x1 {
                let width = (x1 - x0).max(1e-5);
                return y0 + (y1 - y0) * (x - x0) / width + self.bias;
            }
        }
        last.1 + self.bias
    }
}

/// The four bindings of an emitter: `P[1]` to `P[4]` (the executable writes `P[1..=4]` and the emitters read `P[0]` as well,
/// which nothing here writes; taken as 1, **inferred**).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bindings {
    pub bindings: [Binding; 4],
}

impl Emitter {
    fn collision_block(&self, op: u8) -> Option<&[u8]> {
        self.task(op).map(|task| task.block.as_slice())
    }

    pub fn collision_mesh(&self) -> Option<CollisionMesh> {
        let b = self.collision_block(EMITTER_COLLISION_MESH)?;
        let pair = |at: usize| Some([f32_at(b, at)?, f32_at(b, at + 4)?]);
        let byte = |at: usize| b.get(at).copied();
        Some(CollisionMesh {
            speed: pair(0)?,
            angle: [pair(0x08)?, pair(0x10)?],
            window: pair(0x18)?,
            fade_distance: pair(0x20)?,
            radius: f32_at(b, 0x48)?,
            height: f32_at(b, 0x4C)?,
            facing: f32_at(b, 0x50)?,
            rate: u32_at(b, 0x54)?,
            burst: u32_at(b, 0x58)?,
            params: [byte(0x5C)?, byte(0x5D)?, byte(0x5E)?, byte(0x5F)?, byte(0x60)?],
            quality: byte(0x61)?,
        })
    }

    pub fn bounce(&self) -> Option<Bounce> {
        let b = self.collision_block(UPDATE_POS_VEL_WITH_BOUNCE_PLANE)?;
        Some(Bounce {
            restitution: [f32_at(b, 0x1C)?, f32_at(b, 0x20)?],
            friction: [f32_at(b, 0x24)?, f32_at(b, 0x28)?],
            gravity: f32_at(b, 0x2C)?,
            drag: f32_at(b, 0x30)?,
            wind_blend: f32_at(b, 0x34)?,
            turbulence: f32_at(b, 0x38)?,
        })
    }

    pub fn velocity_streak(&self) -> Option<VelocityStreak> {
        let b = self.collision_block(RENDER_SPRITE_VELOCITY)?;
        Some(VelocityStreak {
            length: [f32_at(b, 0x30)?, f32_at(b, 0x34)?],
            length_per_speed: f32_at(b, 0x48)?,
            floor: f32_at(b, 0x40)?,
            opacity: f32_at(b, 0x44)?,
            scale_param: *b.get(0x4C)?,
        })
    }

    pub fn bindings(&self) -> Option<Bindings> {
        let b = self.collision_block(UPDATE_BINDINGS)?;
        let unit = |v: u8| f32::from(v) / 255.0;
        let mut out = [Binding { source: Source::Constant, scale: 0.0, bias: 0.0, curve: [(0.0, 0.0); 5] }; 4];
        for (n, slot) in out.iter_mut().enumerate() {
            let curve = b.get(0x18 + 10 * n..0x18 + 10 * n + 10)?;
            *slot = Binding {
                source: Source::from_byte(*b.get(0x60 + n)?),
                scale: f32_at(b, 0x40 + 4 * n)?,
                bias: f32_at(b, 0x50 + 4 * n)?,
                curve: std::array::from_fn(|i| (unit(curve[5 + i]), unit(curve[i]))),
            };
        }
        Some(Bindings { bindings: out })
    }
}

/// The names of the effect's inputs, in the order the file lists them (file +0x20: a count, three words, one type byte for each
/// input, then the names as C strings). The collision effects have ten: `Transform`, `Velocity`, `ColourOverride`,
/// `VehicleParams`, `PreCrashVelocity`, `CollisionMeshParams`, `IsEmitting`, `MovementSinceLastFrame`,
/// `ImpactClosingVelocity` and `Intensity`.
pub fn input_names(main: &[u8]) -> Vec<String> {
    let Some(count) = u16_at(main, 0x20).map(usize::from) else { return Vec::new() };
    let mut at = 0x28 + count;
    let mut names = Vec::new();
    for _ in 0..count {
        let Some(rest) = main.get(at..) else { break };
        let Some(end) = rest.iter().position(|&byte| byte == 0) else { break };
        names.push(String::from_utf8_lossy(&rest[..end]).into_owned());
        at += end + 1;
    }
    names
}

/// The offset in the main block at which each emitter starts, in order. A resource import whose slot is at or after one
/// offset and before the next belongs to that emitter (an emitter's material sits in the block of its render operation).
pub fn emitter_offsets(main: &[u8]) -> Result<Vec<usize>, Error> {
    // Parsing is the check that the header is an effect; the table is read again here for the offsets.
    Effect::parse(main)?;
    let count = usize::from(u16_at(main, 4).ok_or(Error::Truncated("header"))?);
    let table = usize::from(u16_at(main, 0x12).ok_or(Error::Truncated("header"))?);
    (0..count).map(|n| u16_at(main, table + 2 * n).map(usize::from).ok_or(Error::Truncated("emitter table"))).collect()
}

#[cfg(test)]
#[path = "collision_tests.rs"]
mod tests;
