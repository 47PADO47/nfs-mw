//! Short, unshadowed spill of flashing emergency light bars onto the road and the cars around them. An artistic
//! enhancement: the light only reads the bar's lamps and the flash level its flares already show
//! (`car_light_flares::intensity::samples`), so light and flare pulse together. Values are untuned by eye.
use super::super::car_light_flares::intensity;
use super::super::car_lights::{CarLights, Kind, LampState};
use blackbox_gfx::LocalLight;
use glam::{Mat4, Vec3};

/// Cars with a running light bar further than this from the camera (metres) cast no light.
const DISTANCE: f32 = 150.0;
/// Width (metres) of the fade towards the distance or count boundary, as for traffic beams.
const FADE: f32 = 24.0;
/// Most cars whose bars cast light; each casts at most one light per colour.
pub(super) const CARS: usize = 4;
/// Reach of a colour's light (metres): the road around the car and the cars beside it.
const RANGE: f32 = 14.0;
/// A wide downward cone: about 84 degrees off straight down at its edge, so a light 1.5 m up reaches the road ~14 m
/// out and the sides of neighbouring cars.
const OUTER_COS: f32 = 0.1;
const INNER_COS: f32 = 0.55;
/// Flash levels below this cast nothing.
const DARK: f32 = 0.02;
/// The light sits this far (metres) above the lamps so the bar's own housing does not swallow it.
const LIFT: f32 = 0.15;

/// A light bar's colour, as the light it casts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scenes::world) enum SirenColour {
    Red,
    Blue,
    White,
}

impl SirenColour {
    const ALL: [Self; 3] = [Self::Red, Self::Blue, Self::White];

    /// Linear colour at full flash and strength 1. Kept below the headlight's 1.5 since the cone is far wider.
    fn rgb(self) -> Vec3 {
        match self {
            Self::Red => Vec3::new(1.2, 0.05, 0.03),
            Self::Blue => Vec3::new(0.1, 0.3, 1.3),
            Self::White => Vec3::new(0.8, 0.8, 0.85),
        }
    }
}

/// The colour a lamp's light counts towards; `None` for lamps that are not part of a red, blue or white bar.
pub(in crate::scenes::world) fn siren_colour(kind: Kind) -> Option<SirenColour> {
    match kind {
        Kind::Siren(true) | Kind::SirenBright(true) => Some(SirenColour::Red),
        Kind::Siren(false) | Kind::SirenBright(false) => Some(SirenColour::Blue),
        Kind::SirenWhite => Some(SirenColour::White),
        _ => None,
    }
}

/// One car's running light bar.
pub(in crate::scenes::world) struct LightBar<'a> {
    /// Stable identity, to break distance ties.
    pub key: u32,
    pub lamps: &'a CarLights,
    pub transform: Mat4,
    pub state: LampState,
}

/// Append the light of the nearest `CARS` bars within `DISTANCE` of `camera`, faded towards the selection boundary,
/// stopping at `remaining` lights.
pub(super) fn emit_nearest<'a>(
    bars: impl Iterator<Item = LightBar<'a>>,
    camera: Vec3,
    strength: f32,
    remaining: usize,
    out: &mut Vec<LocalLight>,
) {
    if strength <= 0.0 || remaining == 0 {
        return;
    }
    let mut nearby: Vec<_> = bars
        .map(|bar| (bar.transform.w_axis.truncate().distance(camera), bar))
        .filter(|(distance, _)| *distance < DISTANCE)
        .collect();
    nearby.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.key.cmp(&b.1.key)));
    let boundary = nearby.get(CARS).map_or(DISTANCE, |(distance, _)| *distance);
    let end = out.len() + remaining;
    for (distance, bar) in nearby.into_iter().take(CARS) {
        let fade = ((boundary - distance) / FADE).clamp(0.0, 1.0);
        let fade = fade * fade * (3.0 - 2.0 * fade);
        emit(bar.lamps, bar.transform, bar.state, strength * fade, out);
        if out.len() >= end {
            out.truncate(end);
            return;
        }
    }
}

/// Append one light per bar colour lit this frame: at the lit lamps of that colour (weighted by their flash level),
/// pointing down from the roof, at the brightest lamp's level.
pub(super) fn emit(lamps: &CarLights, transform: Mat4, state: LampState, strength: f32, out: &mut Vec<LocalLight>) {
    if strength <= 0.0 {
        return;
    }
    let up = transform.transform_vector3(Vec3::Z).normalize_or_zero();
    if up == Vec3::ZERO {
        return;
    }
    for colour in SirenColour::ALL {
        let (mut sum, mut weight, mut level) = (Vec3::ZERO, 0.0, 0.0f32);
        for (lamp, lit) in intensity::samples(lamps, state) {
            if siren_colour(lamp.kind) != Some(colour) || lit <= DARK {
                continue;
            }
            sum += lamp.at * lit;
            weight += lit;
            level = level.max(lit.min(1.0));
        }
        if level <= DARK {
            continue;
        }
        out.push(LocalLight {
            position: (transform.transform_point3(sum / weight) + up * LIFT).to_array(),
            direction: (-up).to_array(),
            casts_shadow: false,
            shadow_exclude: None,
            upper_plane: None,
            range: RANGE,
            outer_cos: OUTER_COS,
            inner_cos: INNER_COS,
            color: (colour.rgb() * level * strength).to_array(),
        });
    }
}

#[cfg(test)]
#[path = "light_bars_tests.rs"]
mod tests;
