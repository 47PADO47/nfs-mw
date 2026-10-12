//! The rain drop pool: spawn in a box around the emitter, fall with the wind, recycle at the ground
//! limits. Spec: `docs/specs/weather.md` (the drop pool). The motion constants are not measured yet, so
//! they are in `Tuning` and set by the caller.

/// Drops in the pool, from the spec.
pub const POOL_SIZE: usize = 200;

/// Tunables the spec has not measured. The caller sets them, or the calibration pass does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Fall speed in metres per second.
    pub fall_speed: f32,
    /// Half-size of the spawn box, metres, on each axis.
    pub box_half: [f32; 3],
    /// Ground limits: a drop below `ground_low` or above `ground_high` on the vertical axis is recycled.
    pub ground_low: f32,
    pub ground_high: f32,
    /// How strongly the wind moves a drop, metres per second per unit of wind.
    pub wind_scale: f32,
    /// New drops per second while it rains, per unit of rain intensity.
    pub spawn_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drop {
    pub pos: [f32; 3],
    pub falling: bool,
}

impl Default for Drop {
    fn default() -> Self {
        Self { pos: [0.0; 3], falling: false }
    }
}

/// The fixed-size drop pool.
#[derive(Debug, Clone, PartialEq)]
pub struct DropPool {
    drops: Vec<Drop>,
    /// Spawns owed from earlier frames, so slow rates still spawn over time.
    spawn_credit: f32,
}

impl Default for DropPool {
    fn default() -> Self {
        Self::new()
    }
}

impl DropPool {
    pub fn new() -> Self {
        Self { drops: vec![Drop::default(); POOL_SIZE], spawn_credit: 0.0 }
    }

    /// The drops, falling or not.
    pub fn drops(&self) -> &[Drop] {
        &self.drops
    }

    /// How many drops are falling.
    pub fn falling(&self) -> usize {
        self.drops.iter().filter(|d| d.falling).count()
    }

    /// Advance one frame. `emitter` is the centre of the spawn box, `wind` is the horizontal wind
    /// (x, z), `rain` is the eased rain intensity, and `rand` returns values in -1..=1.
    pub fn update(
        &mut self,
        dt: f32,
        emitter: [f32; 3],
        wind: [f32; 2],
        rain: f32,
        tuning: &Tuning,
        rand: &mut dyn FnMut() -> f32,
    ) {
        for drop in self.drops.iter_mut().filter(|d| d.falling) {
            drop.pos[0] += wind[0] * tuning.wind_scale * dt;
            drop.pos[2] += wind[1] * tuning.wind_scale * dt;
            drop.pos[1] -= tuning.fall_speed * dt;
            if drop.pos[1] < tuning.ground_low || drop.pos[1] > tuning.ground_high {
                drop.falling = false;
            }
        }

        // Spawns happen only while it rains.
        if rain <= 0.0 {
            self.spawn_credit = 0.0;
            return;
        }
        self.spawn_credit += tuning.spawn_rate * rain * dt;
        for drop in self.drops.iter_mut() {
            if self.spawn_credit < 1.0 {
                break;
            }
            if drop.falling {
                continue;
            }
            *drop = spawn(emitter, tuning, rand);
            self.spawn_credit -= 1.0;
        }
    }
}

/// A drop placed at a random point in the spawn box around the emitter.
fn spawn(emitter: [f32; 3], tuning: &Tuning, rand: &mut dyn FnMut() -> f32) -> Drop {
    let mut pos = emitter;
    for (p, half) in pos.iter_mut().zip(tuning.box_half) {
        *p += rand().clamp(-1.0, 1.0) * half;
    }
    Drop { pos, falling: true }
}
