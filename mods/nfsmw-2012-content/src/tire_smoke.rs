//! The tire smoke of the 2012 game as a variant of the 2005 game's: the smoke effect the road surfaces name for a tyre, written
//! as pictures and a `profile.toml` that the game draws (`docs/modding/effect-variants.md`, evidence in
//! `docs/formats/chameleon-effects.md`, "Wheel effects").
//!
//! In the 2012 game every surface (`HAWAII/SURFACELIST.BNDL`, the `SurfaceVfx` and `SurfaceType` objects) names effects for a
//! tyre. The `SurfaceType` object has three effect references: at +0x0C an effect that differs by surface (grey smoke on
//! tarmac, brown dust on dirt), at +0x14 one that every road surface names (seven smoke emitters, the colour of which the
//! effect's `ColourOverride` input replaces) and at +0x1C sparks. This module takes the effect at +0x14 that most surfaces
//! name, which is the tarmac smoke, found by that and by its `EmitterWheel` operations; no resource id of the 2012 game is
//! written here.
//!
//! An emitter of it is a cloud: a lifetime, a radius over the life, a colour gradient and alpha curve, and a wheel spawner that
//! says how many particles a second a smoking tyre makes (`chameleon-effects`, `wheel`). The numbers go into the profile as
//! they are, except those the game cannot be told: see the constants.

use std::path::Path;

use anyhow::{Context, Result};
use chameleon_bundle::{Bundle, ResourceKind};
use chameleon_effects::{Effect, Emitter as FxEmitter};

use crate::source::Source;
use crate::vfx::{self, GLOBAL_EFFECTS, Picture};
use crate::vfx_profile::{choose_keys, colour_at};

/// The id of the variant in the `tire_smoke_variant` option, and the folder name of its files.
pub const VARIANT: &str = "mw2012";
/// What the menu shows for it.
pub const LABEL: &str = "NFS Most Wanted 2012";

/// The file with the surfaces.
const SURFACES: &str = "HAWAII/SURFACELIST.BNDL";
/// Where a `SurfaceType` object keeps its reference to the effect every road surface names.
const SURFACE_EFFECT: u32 = 0x14;
const TYPE_FLAG: u32 = 0x8000_0000;
/// The folder the pictures are named in.
const FOLDER: &str = "TIRESMOKE/MW2012";
/// The most keys one layer is given.
const MAX_KEYS: usize = 8;
/// The age fraction the curves are read at for the end of a life.
const LAST_MOMENT: f32 = 0.9999;
/// A sprite's width in radii.
const WIDTH_PER_RADIUS: f32 = 2.0;
/// Degrees in one unit of the starting rotation range.
const DEGREES_PER_ROTATION_UNIT: f32 = 360.0;
/// The widest a puff is drawn, metres. The biggest cloud of the effect grows to 14 m wide; in the rewrite's chase view that is a
/// wall, so it stops here. **Chosen, not decoded.**
const WIDEST: f32 = 8.0;
/// The share of the decoded rates that is used. The effect's seven emitters each scale their rate by an effect parameter that
/// is bound to the wheel's slip (`UpdateBindings_Wheel`, whose response curves are not evaluated), so they do not all run at
/// full rate at once; at the full rates of the file the clouds fill the screen. **Chosen against the game's own smoke volume,
/// not decoded.**
const RATE_SCALE: f32 = 0.25;
/// An emitter whose particles are smaller than this (metres of radius) is the tyre's grit, not smoke, and is left out.
const SMALLEST_RADIUS: f32 = 0.2;
/// How a puff rises, metres a second squared, and how widely the puffs leave the tyre, degrees. The wheel spawner puts a puff
/// at the tyre with a speed of 0.1 to 0.5 m/s; the file's directions are around the axle and are not carried over.
/// **Chosen, not decoded.**
const RISE: f32 = 0.25;
const SPREAD: f32 = 90.0;
/// The pictures: a square one is a 2 x 2 sheet of puffs; one twice as wide as it is high is a 4 x 2 sheet, which is written as
/// a 4 x 4 sheet (the two rows again below) because a profile's sheets are square. Read off the pictures, not in the data.
const SQUARE_GRID: u8 = 2;
const WIDE_GRID: u8 = 4;

/// The pictures and the text of the profile.
#[derive(Debug, Clone, PartialEq)]
pub struct SmokeSet {
    /// File name and picture, written under `files/TIRESMOKE/MW2012/`.
    pub pictures: Vec<(String, Picture)>,
    /// The `[[layer]]` tables.
    pub toml: String,
}

/// The effect of the tarmac tyre smoke, an id of `GLOBALEFFECTS.BNDL`: the one with wheel emitters that most surface types name at
/// +0x14 (the lowest id when two are named as often). `None` when the install has no such surface list.
pub fn find_effect(surfaces: &Bundle, effects: &Bundle) -> Option<u64> {
    let object = ResourceKind::GenesysObject.id();
    let mut named: Vec<(u64, usize)> = Vec::new();
    for (index, entry) in surfaces.entries().iter().enumerate() {
        if entry.kind != object || entry.import_count == 0 {
            continue;
        }
        let Ok(resource) = surfaces.resource(index) else { continue };
        for import in resource.imports.iter().filter(|i| i.offset == TYPE_FLAG | SURFACE_EFFECT) {
            if !has_wheel_emitters(effects, import.id) {
                continue;
            }
            match named.iter_mut().find(|(id, _)| *id == import.id) {
                Some((_, count)) => *count += 1,
                None => named.push((import.id, 1)),
            }
        }
    }
    named.into_iter().min_by_key(|(id, count)| (std::cmp::Reverse(*count), *id)).map(|(id, _)| id)
}

/// Whether the effect `id` of the bundle has an emitter with a wheel spawner.
fn has_wheel_emitters(effects: &Bundle, id: u64) -> bool {
    let Some(index) = effects.find(id) else { return false };
    if effects.entries()[index].kind != ResourceKind::BearEffect.id() {
        return false;
    }
    let Ok(resource) = effects.resource(index) else { return false };
    let Ok(effect) = Effect::parse(&resource.blocks[0]) else { return false };
    effect.emitters.iter().any(|emitter| emitter.wheel().is_some())
}

/// The tarmac tyre smoke of the install: its pictures and its layers. `None` when the install has none that reads.
pub fn find(source: &Source) -> Option<SmokeSet> {
    let global = source.read_file(GLOBAL_EFFECTS).ok()?;
    let global = Bundle::parse(&global).ok()?;
    let surfaces = source.read_file(SURFACES).ok()?;
    let surfaces = Bundle::parse(&surfaces).ok()?;
    let id = find_effect(&surfaces, &global)?;
    let emitters =
        vfx::emitters(&global, id).map_err(|error| log::warn!("tire smoke effect {id:#x}: {error:#}")).ok()?;
    build(&emitters)
}

/// The layers of the emitters. An emitter that lacks a number the layer is made of, or is only grit, or repeats an earlier one
/// (the same picture, size and life: the effect has one for each of two slip regimes) is left out.
pub fn build(emitters: &[vfx::Emitter]) -> Option<SmokeSet> {
    let mut set = SmokeSet { pictures: Vec::new(), toml: String::new() };
    let mut seen: Vec<(usize, [f32; 4])> = Vec::new();
    for emitter in emitters {
        let Some(layer) = Layer::of(emitter) else { continue };
        let (file, grid) = add_picture(&mut set, &emitter.picture);
        let picture = set.pictures.iter().position(|(name, _)| *name == file).unwrap_or(0);
        let signature = [layer.life.0, layer.life.1, layer.radius.0, layer.radius.1];
        if seen.iter().any(|(known, same)| *known == picture && *same == signature) {
            continue;
        }
        seen.push((picture, signature));
        set.toml.push_str(&layer.toml(&file, grid));
    }
    (!set.toml.is_empty()).then_some(set)
}

/// The file name of a picture, adding the picture once. A 4 x 2 sheet is written as a 4 x 4 one.
fn add_picture(set: &mut SmokeSet, picture: &Picture) -> (String, u8) {
    let (base, grid, picture) = match (picture.width == picture.height, picture.width == 2 * picture.height) {
        (true, _) => ("PUFF", SQUARE_GRID, picture.clone()),
        (_, true) => ("SHEET", WIDE_GRID, stacked(picture)),
        _ => ("SMOKE", 0, picture.clone()),
    };
    let mut n = 1;
    loop {
        let name = if n == 1 { format!("{base}.PNG") } else { format!("{base}{n}.PNG") };
        match set.pictures.iter().find(|(known, _)| *known == name) {
            Some((_, same)) if *same == picture => return (name, grid),
            Some(_) => n += 1,
            None => {
                set.pictures.push((name.clone(), picture));
                return (name, grid);
            }
        }
    }
}

/// A picture twice as wide as it is high, with its rows again below it: a square sheet of the same cells.
fn stacked(picture: &Picture) -> Picture {
    let rgba = picture.rgba.iter().chain(&picture.rgba).copied().collect();
    Picture { width: picture.width, height: picture.height * 2, rgba }
}

/// One `[[layer]]`, as numbers.
#[derive(Debug, Clone, PartialEq)]
struct Layer {
    life: (f32, f32),
    radius: (f32, f32),
    rate: f32,
    rate_per_speed: f32,
    speed: f32,
    speed_variance: f32,
    damping: f32,
    inherit: f32,
    inherit_variance: f32,
    angle_range: f32,
    spin: f32,
    keys: Vec<f32>,
    size: Vec<f32>,
    colors: Vec<[u8; 4]>,
}

impl Layer {
    /// The layer for one emitter, or `None` when it lacks the lifetime, radius, colour or wheel spawner that the numbers come
    /// from, or is grit.
    fn of(emitter: &vfx::Emitter) -> Option<Layer> {
        let fx: &FxEmitter = &emitter.fx;
        let (life, wheel, gradient) = (fx.lifetime()?, fx.wheel()?, fx.gradient()?);
        let (radius, curve) = (fx.radius()?, fx.radius_curve()?);
        let mean = (radius.min + radius.max) * 0.5;
        if mean < SMALLEST_RADIUS {
            return None;
        }
        let width = |t: f32| (WIDTH_PER_RADIUS * mean * curve.at(t)).clamp(0.0, WIDEST);
        let mut knots = gradient.alpha_x.to_vec();
        knots.push(curve.peak);
        knots.extend(gradient.weight_breakpoints());
        let reference = knots.iter().map(|k| width(*k)).fold(f32::MIN_POSITIVE, f32::max);
        let keys = choose_keys(&knots, MAX_KEYS, |t| {
            let [r, g, b, a] = colour_at(&gradient, t).map(|v| f32::from(v) / 255.0);
            [a, (width(t) / reference).clamp(0.0, 1.0), r, g, b]
        });
        let longest = life.max.max(0.01);
        let inherit = fx.inherit().map_or(0.0, |i| i.max.clamp(0.0, 1.0));
        let (low, high) = (wheel.speed[0].min(wheel.speed[1]), wheel.speed[0].max(wheel.speed[1]));
        Some(Layer {
            life: (life.min.min(longest), longest.min(10.0)),
            radius: (radius.min, radius.max),
            rate: (wheel.rate * RATE_SCALE).clamp(0.0, 500.0),
            rate_per_speed: (wheel.rate_per_speed * RATE_SCALE).clamp(0.0, 100.0),
            speed: ((low + high) * 0.5).clamp(-50.0, 50.0),
            speed_variance: ((high - low) * 0.5).clamp(0.0, 50.0),
            damping: fx.bounce_drag().or_else(|| fx.motion().map(|m| m.drag)).unwrap_or(0.0).clamp(0.0, 1.0),
            inherit,
            inherit_variance: match inherit > 0.0 {
                true => fx.inherit().map_or(0.0, |i| ((inherit - i.min) / inherit).clamp(0.0, 1.0)),
                false => 0.0,
            },
            angle_range: fx.rotation().map_or(0.0, |r| ((r.max - r.min).abs() * DEGREES_PER_ROTATION_UNIT).min(360.0)),
            spin: fx.rotation().map_or(0.0, |r| (r.spin.abs() * DEGREES_PER_ROTATION_UNIT).min(3600.0)),
            size: keys.iter().map(|key| width(*key)).collect(),
            colors: keys.iter().map(|key| colour_at(&gradient, key.min(LAST_MOMENT))).collect(),
            keys,
        })
    }

    fn toml(&self, texture: &str, grid: u8) -> String {
        let cells = if grid > 0 { format!("grid = {grid}\nrandom_start = true\nfps = 0\n") } else { String::new() };
        let list = |values: &[f32]| values.iter().map(|v| format!("{v:.4}")).collect::<Vec<_>>().join(", ");
        let colors: Vec<String> =
            self.colors.iter().map(|c| format!("[{}, {}, {}, {}]", c[0], c[1], c[2], c[3])).collect();
        let longest = self.life.1.max(0.01);
        let variance = ((longest - self.life.0) / longest).clamp(0.0, 1.0);
        format!(
            "\n[[layer]]\ntexture = \"{FOLDER}/{texture}\"\nblend = \"alpha\"\n{cells}rate = {:?}\nrate_per_speed = {:?}\n\
             life = {:?}\nlife_variance = {:?}\nspeed = {:?}\nspeed_variance = {:?}\nspread = {SPREAD:?}\nlift = {RISE:?}\n\
             damping = {:?}\ninherit = {:?}\ninherit_variance = {:?}\nangle_range = {:?}\nspin = {:?}\nkeys = [{}]\n\
             size = [{}]\ncolors = [{}]\n",
            self.rate,
            self.rate_per_speed,
            longest.min(10.0),
            variance,
            self.speed,
            self.speed_variance,
            self.damping,
            self.inherit,
            self.inherit_variance,
            self.angle_range,
            self.spin,
            list(&self.keys),
            list(&self.size),
            colors.join(", "),
        )
    }
}

/// Convert the tire smoke of the install into the variant the game's tire smoke option offers. A failure only costs the variant:
/// the cars are converted, and the game's own smoke stays the only choice.
pub fn write_variant(source: &Source, out: &Path) {
    let Some(set) = find(source) else {
        log::warn!("the 2012 install has no tire smoke that can be read: the tire smoke variant has none");
        return;
    };
    if let Err(error) = write(out, &set) {
        log::warn!("the tire smoke variant is not written: {error:#}");
    }
}

/// The `profile.toml` of the pictures and layers of `set`.
pub fn profile_text(set: &SmokeSet) -> String {
    format!("format = 1\n{}", set.toml)
}

/// Write the pictures and the profile under `out/files/TIRESMOKE/<VARIANT>/`.
pub fn write(out: &Path, set: &SmokeSet) -> Result<()> {
    let folder = out.join("files").join("TIRESMOKE").join(VARIANT.to_ascii_uppercase());
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    for (name, picture) in &set.pictures {
        let bytes = vfx::png_bytes(picture)?;
        std::fs::write(folder.join(name), bytes).with_context(|| format!("writing {name}"))?;
    }
    std::fs::write(folder.join("profile.toml"), profile_text(set)).context("writing the tire smoke profile")
}

#[cfg(test)]
mod tests;
