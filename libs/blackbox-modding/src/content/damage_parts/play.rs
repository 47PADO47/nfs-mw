//! The parts of a car that a wing machine or a set of conditions plays out: `[wing]`, `[[damage_parts.track]]` and
//! `[[damage_parts.drive]]`. Spec: `docs/modding/car-damage-parts.md`.

use serde::Deserialize;

use super::{ContentError, DamagePart, MAX_MPH, MAX_TRAVEL, SECONDS_RANGE, field};

/// What a drive test reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveInput {
    /// The speed, mph.
    Speed,
    /// The throttle, 0 to 1.
    Gas,
    /// The brake, 0 to 1.
    Brake,
    /// The steering, -1 to 1 (positive to the right).
    Steering,
}

/// How a drive test compares the input with its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

/// One test of a drive: `input op value`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriveTest {
    pub input: DriveInput,
    pub op: DriveOp,
    pub value: f32,
}

/// A way a part is played out (`[[damage_parts.drive]]`): the part's `[damage_parts.active]` movement runs forward from rest
/// to its full pose once the opening tests hold (and have held `open_after_seconds`), and back once the closing tests hold.
#[derive(Debug, Clone, PartialEq)]
pub struct Drive {
    pub open: Vec<DriveTest>,
    /// One opening test is enough (otherwise all have to hold).
    pub open_any: bool,
    pub open_after_seconds: f32,
    pub close: Vec<DriveTest>,
    /// One closing test is enough (otherwise all have to hold).
    pub close_any: bool,
}

/// Most drives a part has, and most tests in one list.
pub const MAX_DRIVES: usize = 4;
const MAX_TESTS: usize = 6;

/// Animations a wing machine plays.
pub const WING_SLOTS: usize = 5;
/// Most keys of a track.
const MAX_KEYS: usize = 64;

/// A wing machine (`[wing]`): the numbers of the machine that plays a car's five keyed animations over its speed and brake.
#[derive(Debug, Clone, PartialEq)]
pub struct Wing {
    /// The wing opens above this speed (animation 0), mph.
    pub open_mph: f32,
    /// The brake opens it at rest above this speed (animation 1), mph.
    pub brake_mph: f32,
    /// Above this speed it opens further (animation 4), mph. `None`: the wing has no top-speed animation.
    pub top_mph: Option<f32>,
    /// The brake counts once it has been held this long, seconds.
    pub brake_hold_seconds: f32,
    /// How long the cross-fades take, seconds.
    pub fade_seconds: f32,
    /// The length of each animation, seconds.
    pub slot_seconds: [f32; WING_SLOTS],
}

/// The keys of one animation of a part: its slide and its turn at keys spread evenly over the animation.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub slot: usize,
    /// The slide at each key, in the car solid space, metres.
    pub translation: Vec<[f32; 3]>,
    /// The turn at each key as a quaternion (x, y, z, w) in the same space, about the part pivot.
    pub rotation: Vec<[f32; 4]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawTrack {
    slot: usize,
    translation: Vec<[f32; 3]>,
    rotation: Vec<[f32; 4]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawTest {
    input: String,
    op: String,
    value: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawDrive {
    open: Vec<RawTest>,
    #[serde(default)]
    open_any: bool,
    #[serde(default)]
    open_after_seconds: f32,
    close: Vec<RawTest>,
    #[serde(default)]
    close_any: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::content) struct RawWing {
    open_mph: f32,
    brake_mph: f32,
    #[serde(default)]
    top_mph: Option<f32>,
    brake_hold_seconds: f32,
    fade_seconds: f32,
    slot_seconds: [f32; WING_SLOTS],
}

pub(super) fn check_track(raw: RawTrack) -> Result<Track, ContentError> {
    if raw.slot >= WING_SLOTS {
        return Err(field("a track slot is 0 to 4"));
    }
    let keys = raw.translation.len();
    if !(2..=MAX_KEYS).contains(&keys) || raw.rotation.len() != keys {
        return Err(field("a track has 2 to 64 keys, a slide and a turn for each"));
    }
    if raw.translation.iter().flatten().any(|c| !c.is_finite() || c.abs() > MAX_TRAVEL) {
        return Err(field("a track slides up to 5 m"));
    }
    let unit =
        |q: &[f32; 4]| q.iter().all(|c| c.is_finite()) && (q.iter().map(|c| c * c).sum::<f32>() - 1.0).abs() < 0.05;
    if !raw.rotation.iter().all(unit) {
        return Err(field("track turns are unit quaternions (x, y, z, w)"));
    }
    Ok(Track { slot: raw.slot, translation: raw.translation, rotation: raw.rotation })
}

/// Check the `[wing]` table against the parts: a car with tracks needs it, and it needs tracks.
pub(in crate::content) fn check_wing(raw: Option<RawWing>, parts: &[DamagePart]) -> Result<Option<Wing>, ContentError> {
    let tracked = parts.iter().any(|part| !part.tracks.is_empty());
    let Some(raw) = raw else {
        return match tracked {
            true => Err(field("a part with tracks needs a [wing] table")),
            false => Ok(None),
        };
    };
    if !tracked {
        return Err(field("a [wing] table needs a part with tracks"));
    }
    let speeds: Vec<f32> = [Some(raw.brake_mph), Some(raw.open_mph), raw.top_mph].into_iter().flatten().collect();
    if speeds.iter().any(|v| !v.is_finite() || !(0.0..=MAX_MPH).contains(v)) || speeds.windows(2).any(|w| w[0] > w[1]) {
        return Err(field(
            "wing speeds are 0 to 500 mph with brake_mph not above open_mph and open_mph not above top_mph",
        ));
    }
    let times = [raw.brake_hold_seconds, raw.fade_seconds];
    if times.iter().any(|v| !v.is_finite() || !(0.0..=10.0).contains(v)) || raw.fade_seconds <= 0.0 {
        return Err(field("a wing brake hold is 0 to 10 seconds and its fade above 0"));
    }
    if raw.slot_seconds.iter().any(|v| !v.is_finite() || !(SECONDS_RANGE.0..=60.0).contains(v)) {
        return Err(field("a wing animation takes 0.05 to 60 seconds"));
    }
    Ok(Some(Wing {
        open_mph: raw.open_mph,
        brake_mph: raw.brake_mph,
        top_mph: raw.top_mph,
        brake_hold_seconds: raw.brake_hold_seconds,
        fade_seconds: raw.fade_seconds,
        slot_seconds: raw.slot_seconds,
    }))
}

fn check_tests(raw: Vec<RawTest>) -> Result<Vec<DriveTest>, ContentError> {
    if raw.is_empty() || raw.len() > MAX_TESTS {
        return Err(field("a drive list has 1 to 6 tests"));
    }
    raw.into_iter()
        .map(|test| {
            let input = match test.input.as_str() {
                "speed_mph" => DriveInput::Speed,
                "gas" => DriveInput::Gas,
                "brake" => DriveInput::Brake,
                "steering" => DriveInput::Steering,
                _ => return Err(field("a drive test reads speed_mph, gas, brake or steering")),
            };
            let op = match test.op.as_str() {
                "==" => DriveOp::Eq,
                "!=" => DriveOp::Ne,
                ">" => DriveOp::Gt,
                "<" => DriveOp::Lt,
                ">=" => DriveOp::Ge,
                "<=" => DriveOp::Le,
                _ => return Err(field("a drive test compares with ==, !=, >, <, >= or <=")),
            };
            if !test.value.is_finite() || test.value.abs() > 1000.0 {
                return Err(field("a drive test value is a finite number within 1000"));
            }
            Ok(DriveTest { input, op, value: test.value })
        })
        .collect()
}

pub(super) fn check_drive(raw: RawDrive) -> Result<Drive, ContentError> {
    if !raw.open_after_seconds.is_finite() || !(0.0..=60.0).contains(&raw.open_after_seconds) {
        return Err(field("open_after_seconds is 0 to 60"));
    }
    Ok(Drive {
        open: check_tests(raw.open)?,
        open_any: raw.open_any,
        open_after_seconds: raw.open_after_seconds,
        close: check_tests(raw.close)?,
        close_any: raw.close_any,
    })
}
