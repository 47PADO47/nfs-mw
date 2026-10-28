//! The `[screen]` section of a nitrous profile: what the screen and the camera do while the nitrous burns
//! (`docs/modding/nitrous-variants.md`). Every value is the one at full effect; the host mixes it with the level that follows
//! the burn (`BurnFollower`), so a value at its neutral (0 for a strength, 1 for a multiplier) changes nothing.
//!
//! ```toml
//! [screen]
//! vignette = 0.2                # additive vignette (the renderer's speedbreaker mask), 0 to 1
//! radial_blur = 0.3             # the high-speed radial blur, 0 to 1 (the motion blur setting scales it)
//! edge = [0.04, 0.11, 0.09]     # what the frame is multiplied by at a corner of the screen (a darkening, tinted vignette)
//! edge_inner = 0.5              # where that starts, 0 (centre) to 1 (corner)
//! tint = [0.9, 1.0, 1.05]       # what every colour is multiplied by
//! contrast = 1.1                # about mid grey
//! fov = 8.0                     # degrees added to the chase camera's field of view
//! pull_back = 0.4               # metres the chase camera moves back
//! shake = 0.01                  # camera shake, metres
//! shake_rate = 18.0             # shakes a second
//! fade_in = 0.25                # seconds to full effect
//! fade_out = 0.5                # seconds to leave it
//! ```

use serde::Deserialize;

use super::{ContentError, FADE_RANGE, UNIT_RANGE, within};

/// The screen and camera effects that follow the burn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenProfile {
    pub vignette: f32,
    pub radial_blur: f32,
    pub fade_in: f32,
    pub fade_out: f32,
    /// The multiplier of the frame at a corner of the screen at full effect (1: none).
    pub edge: [f32; 3],
    /// Where the edge starts to darken, 0 at the centre to 1 at a corner.
    pub edge_inner: f32,
    /// The multiplier of every colour at full effect.
    pub tint: [f32; 3],
    /// Contrast about mid grey at full effect (1: none).
    pub contrast: f32,
    /// Degrees added to the chase camera's field of view at full effect.
    pub fov: f32,
    /// Metres the chase camera moves back at full effect.
    pub pull_back: f32,
    /// The camera shake at full effect: the size of the shake in metres, and its rate in shakes a second.
    pub shake: f32,
    pub shake_rate: f32,
}

impl Default for ScreenProfile {
    fn default() -> Self {
        Self {
            vignette: 0.0,
            radial_blur: 0.0,
            fade_in: 0.25,
            fade_out: 0.5,
            edge: [1.0; 3],
            edge_inner: 0.5,
            tint: [1.0; 3],
            contrast: 1.0,
            fov: 0.0,
            pull_back: 0.0,
            shake: 0.0,
            shake_rate: DEFAULT_SHAKE_RATE,
        }
    }
}

const DEFAULT_SHAKE_RATE: f32 = 18.0;
const MULTIPLIER_RANGE: std::ops::RangeInclusive<f32> = 0.0..=4.0;
const FOV_RANGE: std::ops::RangeInclusive<f32> = -30.0..=60.0;
const PULL_BACK_RANGE: std::ops::RangeInclusive<f32> = -5.0..=10.0;
const SHAKE_RANGE: std::ops::RangeInclusive<f32> = 0.0..=0.5;
const SHAKE_RATE_RANGE: std::ops::RangeInclusive<f32> = 0.1..=60.0;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawScreen {
    vignette: Option<f32>,
    radial_blur: Option<f32>,
    fade_in: Option<f32>,
    fade_out: Option<f32>,
    edge: Option<[f32; 3]>,
    edge_inner: Option<f32>,
    tint: Option<[f32; 3]>,
    contrast: Option<f32>,
    fov: Option<f32>,
    pull_back: Option<f32>,
    shake: Option<f32>,
    shake_rate: Option<f32>,
}

fn multipliers(value: Option<[f32; 3]>, field: &'static str) -> Result<[f32; 3], ContentError> {
    let value = value.unwrap_or([1.0; 3]);
    if value.iter().any(|c| !c.is_finite() || !MULTIPLIER_RANGE.contains(c)) {
        return Err(ContentError::Field { field, reason: "three numbers from 0 to 4" });
    }
    Ok(value)
}

pub(super) fn parse(raw: RawScreen) -> Result<ScreenProfile, ContentError> {
    Ok(ScreenProfile {
        vignette: within(raw.vignette, 0.0, &UNIT_RANGE, "screen.vignette")?,
        radial_blur: within(raw.radial_blur, 0.0, &UNIT_RANGE, "screen.radial_blur")?,
        fade_in: within(raw.fade_in, 0.25, &FADE_RANGE, "screen.fade_in")?,
        fade_out: within(raw.fade_out, 0.5, &FADE_RANGE, "screen.fade_out")?,
        edge: multipliers(raw.edge, "screen.edge")?,
        edge_inner: within(raw.edge_inner, 0.5, &UNIT_RANGE, "screen.edge_inner")?,
        tint: multipliers(raw.tint, "screen.tint")?,
        contrast: within(raw.contrast, 1.0, &MULTIPLIER_RANGE, "screen.contrast")?,
        fov: within(raw.fov, 0.0, &FOV_RANGE, "screen.fov")?,
        pull_back: within(raw.pull_back, 0.0, &PULL_BACK_RANGE, "screen.pull_back")?,
        shake: within(raw.shake, 0.0, &SHAKE_RANGE, "screen.shake")?,
        shake_rate: within(raw.shake_rate, DEFAULT_SHAKE_RATE, &SHAKE_RATE_RANGE, "screen.shake_rate")?,
    })
}

/// `neutral` moved towards `full` by `level` (0 to 1).
fn mix(neutral: f32, full: f32, level: f32) -> f32 {
    neutral + (full - neutral) * level.clamp(0.0, 1.0)
}

impl ScreenProfile {
    /// Whether the profile changes the colour of the frame (its edge, tint or contrast): the host then has a grade to draw.
    pub fn has_grade(&self) -> bool {
        self.edge != [1.0; 3] || self.tint != [1.0; 3] || self.contrast != 1.0
    }

    /// Whether the profile moves the camera (field of view, pull-back or shake).
    pub fn has_camera(&self) -> bool {
        self.fov != 0.0 || self.pull_back != 0.0 || self.shake > 0.0
    }

    /// The edge multiplier at `level` of the effect.
    pub fn edge_at(&self, level: f32) -> [f32; 3] {
        self.edge.map(|c| mix(1.0, c, level))
    }

    /// The colour multiplier at `level` of the effect.
    pub fn tint_at(&self, level: f32) -> [f32; 3] {
        self.tint.map(|c| mix(1.0, c, level))
    }

    /// The contrast at `level` of the effect.
    pub fn contrast_at(&self, level: f32) -> f32 {
        mix(1.0, self.contrast, level)
    }

    /// The field of view added at `level`, in degrees.
    pub fn fov_at(&self, level: f32) -> f32 {
        mix(0.0, self.fov, level)
    }

    /// The pull-back at `level`, in metres.
    pub fn pull_back_at(&self, level: f32) -> f32 {
        mix(0.0, self.pull_back, level)
    }

    /// The camera shake at `time` seconds and `level` of the effect: an offset in the camera's own frame in metres
    /// (right, up, forward), the sum of three sines per axis that never line up, so it does not repeat in a way that shows.
    /// Zero without a shake. Pure, so the host and its tests read the same value.
    pub fn shake_at(&self, time: f32, level: f32) -> [f32; 3] {
        let amplitude = self.shake * level.clamp(0.0, 1.0);
        if amplitude <= 0.0 || !time.is_finite() {
            return [0.0; 3];
        }
        let phase = time * self.shake_rate * std::f32::consts::TAU;
        let wave = |a: f32, b: f32, c: f32| {
            (phase * a).sin() * 0.5 + (phase * b + 1.3).sin() * 0.3 + (phase * c + 4.1).sin() * 0.2
        };
        // The forward axis moves least: a shake that pumps the view in and out reads as a fall in the frame rate.
        [wave(1.0, 1.7, 2.9) * amplitude, wave(1.3, 2.3, 3.1) * amplitude, wave(0.7, 1.9, 2.3) * amplitude * 0.3]
    }
}
