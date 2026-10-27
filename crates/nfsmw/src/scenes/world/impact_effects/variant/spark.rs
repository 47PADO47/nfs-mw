//! One spark of a collision spark variant: how it is born, how it moves and what it looks like at an age
//! (`docs/modding/effect-variants-sparks-xenon.md`). Everything is in render space (z up).

use blackbox_modding::content::SparkLayer;
use glam::Vec3;

/// How far off the surface a spark is born, metres.
const SURFACE_LIFT: f32 = 0.02;

/// Where and how hard a contact throws its sparks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Birth {
    pub point: Vec3,
    /// The surface's normal, pointing out of the surface.
    pub normal: Vec3,
    /// The velocity of the car, which a spark takes the layer's share of.
    pub owner: Vec3,
    /// The strength 0 to 1 from the profile's gate.
    pub strength: f32,
    /// An impact scales the speed by the strength; a scrape changes the rate only.
    pub hit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Spark {
    /// The layer, counted over the impact layers and then the scrape layers.
    pub layer: usize,
    pub point: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub life: f32,
    /// A random 0 to 1 that picks the cell of a sheet.
    pub cell: f32,
}

/// What a spark looks like at its age.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Look {
    /// Full width, metres.
    pub size: f32,
    pub color: [u8; 4],
    /// The overbright multiplier of the colour.
    pub brightness: f32,
}

/// Independent uniform samples in `[0, 1)` from a seed and a slot.
pub(super) fn uniform(seed: u64, slot: u64) -> f32 {
    let mut word = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(slot.wrapping_mul(0xbf58_476d_1ce4_e5b9));
    word = (word ^ (word >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    word = (word ^ (word >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((word ^ (word >> 31)) >> 40) as f32 / 16_777_216.0
}

/// Two unit vectors that, with `normal`, make a frame.
fn perpendicular(normal: Vec3) -> (Vec3, Vec3) {
    let helper = if normal.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let first = normal.cross(helper).normalize_or(Vec3::X);
    (first, normal.cross(first))
}

/// The segment of `keys` that `t` falls in and how far along it (0 to 1). Before the first key it is the start of the first
/// segment, past the last the end of the last.
pub(super) fn segment(keys: &[f32], t: f32) -> (usize, f32) {
    let Some(last) = keys.len().checked_sub(1).filter(|last| *last >= 1) else { return (0, 0.0) };
    if t <= keys[0] {
        return (0, 0.0);
    }
    if t >= keys[last] {
        return (last - 1, 1.0);
    }
    let at = keys.iter().rposition(|key| *key <= t).unwrap_or(0).min(last - 1);
    let span = keys[at + 1] - keys[at];
    let along = if span > 0.0 { (t - keys[at]) / span } else { 0.0 };
    (at, along.clamp(0.0, 1.0))
}

/// The value between entries `at` and `at + 1` of a list of `count` values (`value(i)` reads one); a list that is too short
/// repeats its last value, an empty one is 0.
fn blend(count: usize, (at, along): (usize, f32), value: impl Fn(usize) -> f32) -> f32 {
    let Some(last) = count.checked_sub(1) else { return 0.0 };
    let first = value(at.min(last));
    let second = value((at + 1).min(last));
    first + (second - first) * along
}

impl Spark {
    /// A spark of `layer` (number `index` of the profile) born from `birth`; `seed` picks its random numbers.
    pub fn spawn(layer: &SparkLayer, index: usize, birth: &Birth, seed: u64) -> Self {
        let u = |slot| uniform(seed, slot);
        let (first, second) = perpendicular(birth.normal);
        // A direction inside the cone of `spread` degrees around the normal, spread evenly over its cap.
        let cos = 1.0 - u(0) * (1.0 - layer.spread.to_radians().cos());
        let sin = (1.0 - cos * cos).max(0.0).sqrt();
        let azimuth = u(1) * std::f32::consts::TAU;
        let direction = birth.normal * cos + (first * azimuth.cos() + second * azimuth.sin()) * sin;
        let reach = if birth.hit { birth.strength } else { 1.0 };
        let speed = layer.speed * reach * (1.0 - layer.speed_variance * u(2));
        let carried = layer.inherit * (1.0 - layer.inherit_variance * u(3));
        Self {
            layer: index,
            point: birth.point + birth.normal * SURFACE_LIFT,
            velocity: direction * speed + birth.owner * carried,
            age: 0.0,
            life: layer.life * (1.0 - layer.life_variance * u(4)),
            cell: u(5),
        }
    }

    /// One step of `dt` seconds: drag (a share of the speed lost for each m/s it goes), gravity, then the move.
    pub fn step(&mut self, layer: &SparkLayer, dt: f32) {
        self.age += dt;
        self.velocity *= (1.0 - dt * layer.drag * self.velocity.length()).max(0.0);
        self.velocity.z -= layer.gravity * dt;
        self.point += self.velocity * dt;
    }

    pub fn alive(&self) -> bool {
        self.age < self.life && self.point.is_finite()
    }

    /// The size, colour and overbright of the spark now, in straight lines between the layer's keys.
    pub fn look(&self, layer: &SparkLayer) -> Look {
        let at = segment(&layer.keys, (self.age / self.life).clamp(0.0, 1.0));
        let channel = |index: usize| {
            blend(layer.colors.len(), at, |i| f32::from(layer.colors[i][index])).round().clamp(0.0, 255.0) as u8
        };
        Look {
            size: blend(layer.size.len(), at, |i| layer.size[i]).max(0.0),
            color: [channel(0), channel(1), channel(2), channel(3)],
            brightness: blend(layer.brightness.len(), at, |i| layer.brightness[i]).max(0.0),
        }
    }

    /// The part of the layer's picture the spark shows: the whole picture, or a cell of its sheet (a random one, or the frame
    /// of the animation).
    pub fn window(&self, layer: &SparkLayer) -> [[f32; 2]; 4] {
        let side = u32::from(layer.grid);
        if side == 0 {
            return [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        }
        let cells = side * side;
        let index = match (layer.fps, layer.random_frame) {
            (0, true) => (self.cell * cells as f32) as u32 % cells,
            (0, false) => 0,
            (fps, _) => (self.age * f32::from(fps)) as u32 % cells,
        };
        let (x, y) = ((index % side) as f32 / side as f32, (index / side) as f32 / side as f32);
        let step = 1.0 / side as f32;
        [[x, y], [x + step, y], [x + step, y + step], [x, y + step]]
    }
}

#[cfg(test)]
#[path = "spark_tests.rs"]
mod tests;
