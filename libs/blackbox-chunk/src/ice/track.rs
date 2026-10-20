//! Typed ICE camera tracks and the camera one evaluates to. Layouts and rules: `docs/specs/pursuit-breaker-moment.md`.
//!
//! A track is a camera record of a pack ([`CameraRecord`]): a 0x28-byte header (start, length, element count, name)
//! and `N` keys of 0x84 bytes. A key describes one interval of the track: values at its start and at its end. The
//! interval runs from the key's own time to the next key's time (the end of the track for the last key). Everything
//! in the interval is a cubic Hermite curve, or a straight line when the key's cubic flag is off.

use super::bytes::{f32_at, i16_at};
use super::camera::{CAMERA_ELEMENT_LEN, CameraRecord};

/// Name field of the record header: 14 bytes at 0x17, NUL padded.
const NAME_OFFSET: usize = 0x17;
const NAME_LEN: usize = 14;
const START_OFFSET: usize = 0x0C;
const LENGTH_OFFSET: usize = 0x10;
const COUNT_OFFSET: usize = 0x14;

/// The coordinate space of an eye or look point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// Relative to a car.
    Car,
    /// World coordinates.
    World,
    /// World orientation, car origin.
    Hybrid,
    /// Relative to the scene's origin and heading (the NIS scene matrices).
    Scene,
}

impl Space {
    fn from_byte(b: u8) -> Option<Self> {
        Some(match b {
            0 => Self::Car,
            1 => Self::World,
            2 => Self::Hybrid,
            3 => Self::Scene,
            _ => return None,
        })
    }
}

/// One key (0x84 bytes) of a track.
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    /// Byte 0. Zero means the key has no tangents (the curve is flat at both ends).
    pub kind: u8,
    pub smooth: bool,
    /// Eye and dutch/lens use a cubic curve (else a straight line).
    pub cubic_eye: bool,
    /// Look point uses a cubic curve (else a straight line).
    pub cubic_look: bool,
    pub space_eye: Space,
    pub space_look: Space,
    /// 0x10: where the key's interval starts, 0 to 1 along the track.
    pub time: f32,
    /// 0x14: how much of the interval's chord the curve takes as its tangent at the start and at the end.
    pub tangent_length: [f32; 2],
    /// 0x1C: eye at the start and at the end of the interval.
    pub eye: [[f32; 3]; 2],
    /// 0x34: look point at the start and at the end.
    pub look: [[f32; 3]; 2],
    /// 0x4C.
    pub dutch: [f32; 2],
    /// 0x54: lens in millimetres.
    pub lens: [f32; 2],
    /// 0x5C.
    pub near_clip: [f32; 2],
    /// 0x7C, 0x7E, 0x80: depth-of-field aperture, letterbox percent and simulation speed percent.
    pub aperture: [u8; 2],
    pub letterbox: [u8; 2],
    pub sim_speed: [u8; 2],
}

impl Key {
    /// Reads one key from its 0x84 bytes.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < CAMERA_ELEMENT_LEN {
            return None;
        }
        let f = |at: usize| f32_at(bytes, at);
        let pair = |at: usize| Some([f(at)?, f(at + 4)?]);
        let point = |at: usize| Some([f(at)?, f(at + 4)?, f(at + 8)?]);
        Some(Self {
            kind: bytes[0],
            smooth: bytes[1] != 0,
            cubic_eye: bytes[2] != 0,
            cubic_look: bytes[3] != 0,
            space_eye: Space::from_byte(bytes[4])?,
            space_look: Space::from_byte(bytes[5])?,
            time: f(0x10)?,
            tangent_length: pair(0x14)?,
            eye: [point(0x1C)?, point(0x28)?],
            look: [point(0x34)?, point(0x40)?],
            dutch: pair(0x4C)?,
            lens: pair(0x54)?,
            near_clip: pair(0x5C)?,
            aperture: [bytes[0x7C], bytes[0x7D]],
            letterbox: [bytes[0x7E], bytes[0x7F]],
            sim_speed: [bytes[0x80], bytes[0x81]],
        })
    }
}

/// One camera track.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    /// `Track 0`, `Track 1`...
    pub name: String,
    pub start: f32,
    /// Seconds the track plays.
    pub length: f32,
    pub keys: Vec<Key>,
}

impl Track {
    /// Reads a record. `None` when a key does not parse or the header is short.
    pub fn from_record(record: &CameraRecord<'_>) -> Option<Self> {
        let header = record.header;
        let name_bytes = header.get(NAME_OFFSET..NAME_OFFSET + NAME_LEN)?;
        let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
        let count = usize::try_from(i16_at(header, COUNT_OFFSET)?).ok()?;
        if count != record.elements.len() {
            return None;
        }
        Some(Self {
            name: String::from_utf8_lossy(&name_bytes[..name_end]).into_owned(),
            start: f32_at(header, START_OFFSET)?,
            length: f32_at(header, LENGTH_OFFSET)?,
            keys: record.elements.iter().map(|e| Key::parse(e)).collect::<Option<Vec<_>>>()?,
        })
    }
}

/// One cubic Hermite segment: values `p0`, `p1` and tangents `m0`, `m1` over `u` in 0 to 1.
#[derive(Debug, Clone, Copy)]
struct Hermite {
    p0: f32,
    p1: f32,
    m0: f32,
    m1: f32,
}

impl Hermite {
    /// `a u^3 + b u^2 + c u + d`, with the coefficients the engine builds.
    fn cubic(self, u: f32) -> f32 {
        let chord = self.p1 - self.p0;
        let a = (self.m0 + self.m1) - (chord + chord);
        let b = (chord * 3.0 - self.m1) - (self.m0 + self.m0);
        ((a * u + b) * u + self.m0) * u + self.p0
    }

    fn line(self, u: f32) -> f32 {
        (1.0 - u) * self.p0 + u * self.p1
    }

    fn at(self, u: f32, cubic: bool) -> f32 {
        match cubic {
            true => self.cubic(u),
            false => self.line(u),
        }
    }
}

/// What a track says about the camera at one moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSample {
    pub eye: [f32; 3],
    pub look: [f32; 3],
    pub eye_space: Space,
    pub look_space: Space,
    /// Roll as a 16-bit angle (65536 per turn): the dutch value cut to an integer, as the engine does.
    pub roll: u16,
    /// Lens in millimetres.
    pub lens: f32,
    /// Whole field of view as a 16-bit angle: twice the half angle whose tangent is 15.96 over the lens.
    pub fov: u16,
    /// Near plane, metres, when the key sets one.
    pub near_clip: Option<f32>,
    /// World speed factor, 0.01 to 1.
    pub sim_speed: f32,
    /// Letterbox bar height as a fraction of the screen (the percent times 0.00125).
    pub letterbox: f32,
}

/// Half the sensor width of the lens model, in millimetres (`fov = 2 atan(SENSOR / lens)`).
pub const SENSOR_HALF_WIDTH: f32 = 15.96;
const TURN: f32 = 65536.0;

/// The whole field of view of `lens` millimetres as a 16-bit angle.
pub fn fov_of_lens(lens: f32) -> u16 {
    let half = (SENSOR_HALF_WIDTH / lens.max(1e-3)).atan() / std::f32::consts::TAU * TURN;
    (((half as i32) & 0x7FFF) << 1) as u16
}

impl Track {
    /// The interval that holds `t`: its key index and its start and end. `t` outside 0 to 1 has no camera. The key is
    /// the last whose time is not after `t` (the first when none is).
    fn interval(&self, t: f32) -> Option<(usize, f32, f32)> {
        if !(0.0..=1.0).contains(&t) || self.keys.is_empty() {
            return None;
        }
        let index = self.keys.iter().rposition(|k| k.time <= t).unwrap_or(0);
        let start = self.keys[index].time;
        let end = self.keys.get(index + 1).map_or(1.0, |k| k.time);
        Some((index, start, end))
    }

    /// The camera at `t` (0 to 1 along the track). `None` outside the track.
    ///
    /// Tangents: a key's tangent at each end is its chord times the key's tangent length, the whole story for a
    /// track whose key has no continuing neighbour (all the breaker moments are one key). A key of kind 0 has none.
    /// Smoothing across neighbouring keys is not applied.
    pub fn sample(&self, t: f32) -> Option<CameraSample> {
        let (index, start, end) = self.interval(t)?;
        let key = &self.keys[index];
        let span = end - start;
        let u = match span.abs() > 1e-4 {
            true => (t - start) / span,
            false => 0.0,
        };
        let slope = |k: usize| match key.kind {
            0 => 0.0,
            _ => key.tangent_length[k],
        };
        let curve = |p0: f32, p1: f32, cubic: bool| {
            Hermite { p0, p1, m0: (p1 - p0) * slope(0), m1: (p1 - p0) * slope(1) }.at(u, cubic)
        };
        let axes = |a: &[[f32; 3]; 2], cubic: bool| std::array::from_fn(|i| curve(a[0][i], a[1][i], cubic));
        let flat = |p0: f32, p1: f32| Hermite { p0, p1, m0: 0.0, m1: 0.0 }.at(u, true);
        let lens = curve(key.lens[0], key.lens[1], key.cubic_eye);
        let sim = flat(f32::from(key.sim_speed[0]), f32::from(key.sim_speed[1]));
        let letterbox = flat(f32::from(key.letterbox[0]), f32::from(key.letterbox[1]));
        let near = flat(key.near_clip[0], key.near_clip[1]);
        Some(CameraSample {
            eye: axes(&key.eye, key.cubic_eye),
            look: axes(&key.look, key.cubic_look),
            eye_space: key.space_eye,
            look_space: key.space_look,
            roll: (curve(key.dutch[0], key.dutch[1], key.cubic_eye) as i32) as u16,
            lens,
            fov: fov_of_lens(lens),
            near_clip: (near >= 0.0).then_some(near),
            sim_speed: (sim.clamp(0.0, 100.0) * 0.01).max(0.01),
            letterbox: letterbox.max(0.0) * 0.00125,
        })
    }
}

#[cfg(test)]
#[path = "track_tests.rs"]
mod tests;
