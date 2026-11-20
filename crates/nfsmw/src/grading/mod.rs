//! Colour grading of the world with the cubes of a mod's variant (`docs/specs/colour-grading.md`).
//!
//! The player picks a look with the `grading_variant` option a mod adds values to; its profile (`GRADING/<value>/profile.toml`,
//! `blackbox_modding::content::GradingProfile`) says which cube is in force at which hour for the clear and the rain
//! weather. [`GradingLook`] is that profile with its cubes read from the install (`Bridge::grading_look`), and [`Selector`]
//! turns it, every frame, into the [`ColourGrade`] the renderer draws: the cubes of the hour before and after the clock's
//! hour, blended on the GPU; when the weather is between dry and wet the four cubes are blended on the CPU into one
//! (a bake that happens only when the blend moves a step, not every frame).

use std::sync::Arc;

use blackbox_gfx::{ColourCube, ColourGrade};
use blackbox_modding::content::{GradingPair, GradingProfile, GradingWeather, decode_cube};

/// The steps the weather's blend is quantised to for a bake (a bake is 4096 lookups of four cubes: a few hundred microseconds).
const WEATHER_STEPS: f32 = 64.0;
/// The steps the time between two keys is quantised to for a bake.
const HOUR_STEPS: f32 = 32.0;

/// A grading profile with its cubes read.
#[derive(Debug)]
pub struct GradingLook {
    pub profile: GradingProfile,
    /// The cubes of `profile.cubes`, in the same order.
    pub cubes: Vec<Arc<ColourCube>>,
}

impl GradingLook {
    /// Read the cubes `profile` names through `read` (a path of the install's layout to its bytes). A cube that is missing or does
    /// not decode refuses the whole look, with the reason.
    pub fn load(profile: GradingProfile, read: impl Fn(&str) -> std::io::Result<Vec<u8>>) -> Result<Self, String> {
        let mut cubes = Vec::with_capacity(profile.cubes.len());
        for cube in &profile.cubes {
            let bytes = read(&cube.file).map_err(|error| format!("{}: {error}", cube.file))?;
            let data = decode_cube(&cube.file, &bytes).map_err(|error| format!("{}: {error}", cube.file))?;
            let cube = ColourCube::new(data.size, data.rgb).ok_or_else(|| format!("{}: not a cube", cube.file))?;
            cubes.push(Arc::new(cube));
        }
        Ok(Self { profile, cubes })
    }
}

/// How wet the weather is, 0 (dry) to 1: the rain falling or the wet the roads keep after it, whichever is more. Cloud alone
/// does not make it wet.
pub fn wet_factor(rain: f32, road_wetness: f32) -> f32 {
    let unit = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
    unit(rain).max(unit(road_wetness))
}

/// What a bake was made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BakeKey {
    clear: (usize, usize, u8),
    rain: (usize, usize, u8),
    weather: u8,
}

/// Picks the grade for a frame and keeps the last bake.
#[derive(Default)]
pub struct Selector {
    baked: Option<(BakeKey, Arc<ColourCube>)>,
    /// The look the bake belongs to (a bake of another look is dropped).
    look: Option<std::sync::Weak<GradingLook>>,
}

impl Selector {
    /// The grade for `hour` and the `wet` factor at `strength` (0 to 1), `None` when nothing is to be drawn.
    pub fn grade(&mut self, look: &Arc<GradingLook>, hour: f32, wet: f32, strength: f32) -> Option<ColourGrade> {
        if !strength.is_finite() || strength <= 0.0 {
            return None;
        }
        let strength = strength.min(1.0);
        let same = self.look.as_ref().and_then(std::sync::Weak::upgrade).is_some_and(|known| Arc::ptr_eq(&known, look));
        if !same {
            self.baked = None;
            self.look = Some(Arc::downgrade(look));
        }
        let profile = &look.profile;
        let direct = |pair: GradingPair| -> Option<ColourGrade> {
            let cube = |at: usize| look.cubes.get(at).cloned();
            Some(ColourGrade { from: cube(pair.from)?, to: Some(cube(pair.to)?), mix: pair.mix, strength })
        };
        // A profile with one weather's set looks the same in every weather.
        let (Some(clear_set), Some(rain_set)) = (profile.set(GradingWeather::Clear), profile.set(GradingWeather::Rain))
        else {
            let set = profile.set(GradingWeather::Clear).or_else(|| profile.set(GradingWeather::Rain))?;
            return direct(set.at(hour)?);
        };
        let (clear, rain) = (clear_set.at(hour)?, rain_set.at(hour)?);
        let weather = if wet.is_finite() { wet.clamp(0.0, 1.0) } else { 0.0 };
        if weather >= 1.0 - 0.5 / WEATHER_STEPS {
            return direct(rain);
        }
        if weather < 0.5 / WEATHER_STEPS {
            return direct(clear);
        }
        let step = |v: f32, n: f32| (v * n).round() as u8;
        let key = BakeKey {
            clear: (clear.from, clear.to, step(clear.mix, HOUR_STEPS)),
            rain: (rain.from, rain.to, step(rain.mix, HOUR_STEPS)),
            weather: step(weather, WEATHER_STEPS),
        };
        let baked = match &self.baked {
            Some((known, cube)) if *known == key => cube.clone(),
            _ => {
                let cube = Arc::new(bake(look, &key)?);
                self.baked = Some((key, cube.clone()));
                cube
            }
        };
        Some(ColourGrade::single(baked, strength))
    }
}

/// The cube of a bake: the four cubes around the hour, in both weathers, weighted.
fn bake(look: &GradingLook, key: &BakeKey) -> Option<ColourCube> {
    let w = f32::from(key.weather) / WEATHER_STEPS;
    let part = |pair: (usize, usize, u8), share: f32, parts: &mut Vec<(Arc<ColourCube>, f32)>| {
        let mix = f32::from(pair.2) / HOUR_STEPS;
        parts.push((look.cubes.get(pair.0)?.clone(), share * (1.0 - mix)));
        parts.push((look.cubes.get(pair.1)?.clone(), share * mix));
        Some(())
    };
    let mut parts = Vec::with_capacity(4);
    part(key.clear, 1.0 - w, &mut parts)?;
    part(key.rain, w, &mut parts)?;
    let refs: Vec<(&ColourCube, f32)> = parts.iter().map(|(cube, weight)| (cube.as_ref(), *weight)).collect();
    ColourCube::blend(&refs)
}

#[cfg(test)]
mod tests;
