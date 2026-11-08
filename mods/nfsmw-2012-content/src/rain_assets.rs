//! The rain splash and the rain drops of the lens as variants of the 2005 game's rain, and the wet road look
//! (`docs/formats/chameleon-rain-water.md` says what the 2012 install holds and what it does not).
//!
//! **The 2012 game has no rain**: no weather, no streaks, no drops on the lens and no splash on the road was found in its 214
//! effects, its environment or its shaders; its sprinklers and hydrants (`WithinWaterSpray`) and wet tyre tracks are the nearest
//! water. These variants are therefore *made from the 2012 water pictures*, not decoded from a rain effect:
//!
//! - the **rain splash** draws the ring picture of the 2012 water splash (`RING_TEXTURE`, an expanding ring on the ground) and the
//!   water spray column (`SPRAY_TEXTURE`, a short upright burst) where the rain meets the road;
//! - the **rain drops** are the 2012 soft round dots (`DOTS_TEXTURE`, the bokeh sheet of its lamp glow effect) turned into the
//!   displacement picture of one drop on the lens (`RAINDROPPC`), declared as a replacement set so the 2005 game's own lens
//!   droplets refract through it;
//! - the **wet road** is a small set of chosen numbers (`wet_road_profile`), since the 2012 wet look is an engine constant
//!   (`WetScaleParams`) that is not decoded.
//!
//! The pictures are found by their resource ids, which were identified by looking at the pictures and at the effects that use them;
//! each is read by ranges (a few KiB of a 4.8 MB bundle).

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::source::Source;
use crate::trees::ranges;
use crate::vfx::{self, GLOBAL_EFFECTS, Picture};

/// The id of the variants in their options, and what the menu shows.
pub const VARIANT: &str = "mw2012";
pub const LABEL: &str = "NFS Most Wanted 2012";
/// The folders of the variants under the mod's `files/`.
pub const SPLASH_FOLDER: &str = "RAINSPLASH/MW2012";
pub const WET_ROAD_FOLDER: &str = "WETROAD/MW2012";
pub const DROPS_FOLDER: &str = "REPLACE/RAIN_DROPS";
/// The 2005 texture the lens droplets read (`docs/specs/rain-pc.md`).
pub const LENS_TARGET: &str = "RAINDROPPC";

/// `GLOBALEFFECTS.BNDL` textures: the ring of the water splash (64 x 64, used by the effect `0x24F39`), the water spray column
/// (256 x 256, the effects `0x80AFC` and `0x1D6FEC`) and the round dots (256 x 256, the lamp effects `0x7A95F` and `0xAF168`).
const RING_TEXTURE: u64 = 0x0100_0000_0002_512D;
const SPRAY_TEXTURE: u64 = 0x0100_0000_0006_9D24;
const DOTS_TEXTURE: u64 = 0x0100_0000_0007_A956;

/// The size of the lens drop's picture, pixels a side.
const DROP_SIZE: u32 = 64;
/// How strongly the drop bends the picture behind it (the displacement at its rim, 0 to 0.5 of the picture).
const DROP_BEND: f32 = 0.42;

/// The pictures the rain variants are made of.
#[derive(Debug, Clone, PartialEq)]
pub struct RainPictures {
    pub ring: Picture,
    pub spray: Picture,
    pub drop: Picture,
}

/// Read the three pictures by ranges.
pub fn find(source: &Source) -> Result<(RainPictures, u64)> {
    let (effects, reader) = ranges::open(&source.root().join(GLOBAL_EFFECTS))?;
    let ring = whitened(vfx::texture_picture(&effects, RING_TEXTURE).context("the water splash ring")?);
    let spray = whitened(vfx::texture_picture(&effects, SPRAY_TEXTURE).context("the water spray")?);
    let dots = vfx::texture_picture(&effects, DOTS_TEXTURE).context("the round dots")?;
    let drop = lens_drop(&dots).context("making a lens drop out of the round dots")?;
    Ok((RainPictures { ring, spray, drop }, reader.bytes_read()))
}

/// The picture with a dark colour made white, its alpha kept: the 2012 ring is black with an alpha (the effect tints it), and the profile's
/// `color` tints the splash here.
fn whitened(mut picture: Picture) -> Picture {
    let pixels = picture.rgba.len() / 4;
    let mean = picture
        .rgba
        .chunks_exact(4)
        .map(|pixel| u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]))
        .sum::<u64>()
        / (3 * pixels.max(1)) as u64;
    if mean < 64 {
        picture.rgba.chunks_exact_mut(4).for_each(|pixel| pixel[..3].fill(255));
    }
    picture
}

/// The profile of the rain splash: the ring that grows on the road and the spray that stands up from it. **Chosen, not decoded**:
/// every number (the rates, the sizes, the lives, the opacities, the tint of the water).
pub fn splash_profile_text() -> String {
    format!(
        "format = 1\nradius = 10.0\ndensity = 1.0\n\n\
         [[layer]]\ntexture = \"{SPLASH_FOLDER}/RING.PNG\"\nkind = \"ring\"\nblend = \"alpha\"\nrate = 140.0\nlife = 0.55\n\
         life_variance = 0.35\nsize = [0.06, 0.55]\nopacity = 0.55\ncolor = [215, 225, 240]\n\n\
         [[layer]]\ntexture = \"{SPLASH_FOLDER}/SPRAY.PNG\"\nkind = \"burst\"\nblend = \"alpha\"\nrate = 35.0\nlife = 0.3\n\
         life_variance = 0.3\nsize = [0.06, 0.2]\nopacity = 0.4\ncolor = [225, 232, 245]\n"
    )
}

/// The wet road variant's numbers. **Chosen, not decoded**: a road that soaks quicker, shows a little more film and pools a little
/// more, and dries slower than the game's own, as the 2012 game's all-glossy streets look.
pub fn wet_road_profile_text() -> &'static str {
    "format = 1\nfilm = 1.25\npooling = 1.3\nsoak = 1.5\ndry = 0.7\n"
}

/// The replacement set that gives the lens droplets the 2012 drop (`docs/modding/api-replace.md`).
pub fn drops_manifest() -> String {
    format!(
        "format = 1\n\n[set]\nkind = \"rain_drops\"\nkind_label = \"Rain drops\"\nvalue = \"{VARIANT}\"\nvalue_label = \"{LABEL}\"\n\n\
         [[texture]]\ntarget = \"{LENS_TARGET}\"\nfile = \"{DROPS_FOLDER}/{LENS_TARGET}.png\"\nformat = \"rgba8\"\n"
    )
}

/// One drop on the lens, as a displacement picture: green and blue are where a pixel reads the scene (neutral 0.5), alpha the drop's
/// body. The drop's outline is the largest of the round dots of `dots` (found as the biggest connected blob of its alpha, grown
/// until the alpha crosses half); inside it the picture is bent towards the opposite side like the image seen through a drop,
/// the more the nearer the rim.
pub fn lens_drop(dots: &Picture) -> Result<Picture> {
    let (width, height) = (dots.width as usize, dots.height as usize);
    let alpha = |x: usize, y: usize| dots.rgba[(y * width + x) * 4 + 3];
    let bright = |x: usize, y: usize| {
        let at = (y * width + x) * 4;
        u32::from(dots.rgba[at]) + u32::from(dots.rgba[at + 1]) + u32::from(dots.rgba[at + 2])
    };
    // A dot is where the picture is opaque and bright; the picture is read by connected blobs of such pixels.
    let on = |x: usize, y: usize| alpha(x, y) > 128 && bright(x, y) > 384;
    let mut seen = vec![false; width * height];
    let mut best: Option<(usize, f32, f32, f32)> = None;
    for start in 0..width * height {
        let (sx, sy) = (start % width, start / width);
        if seen[start] || !on(sx, sy) {
            continue;
        }
        let mut stack = vec![(sx, sy)];
        seen[start] = true;
        let (mut count, mut sum_x, mut sum_y) = (0usize, 0.0f32, 0.0f32);
        while let Some((x, y)) = stack.pop() {
            count += 1;
            sum_x += x as f32;
            sum_y += y as f32;
            for (nx, ny) in [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)] {
                if nx < width && ny < height && !seen[ny * width + nx] && on(nx, ny) {
                    seen[ny * width + nx] = true;
                    stack.push((nx, ny));
                }
            }
        }
        if best.is_none_or(|(most, ..)| count > most) {
            let radius = (count as f32 / std::f32::consts::PI).sqrt();
            best = Some((count, sum_x / count as f32, sum_y / count as f32, radius));
        }
    }
    let Some((_, cx, cy, radius)) = best.filter(|(count, ..)| *count >= 12) else {
        bail!("no round dot in the picture")
    };
    // The drop fills the picture with a margin: pixel (u, v) of it is the point of the dots' picture `radius * 1.25` from the centre.
    let size = DROP_SIZE as usize;
    let mut rgba = vec![0u8; size * size * 4];
    for py in 0..size {
        for px in 0..size {
            let (dx, dy) = ((px as f32 + 0.5) / size as f32 * 2.0 - 1.0, (py as f32 + 0.5) / size as f32 * 2.0 - 1.0);
            let distance = (dx * dx + dy * dy).sqrt();
            // The body: the dot's own soft edge, sampled where it is read from.
            let (sx, sy) = (cx + dx * radius * 1.25, cy + dy * radius * 1.25);
            let body =
                match (sx >= 0.0 && sy >= 0.0 && (sx as usize) < width && (sy as usize) < height, distance <= 1.0) {
                    (true, true) => f32::from(alpha(sx as usize, sy as usize)) / 255.0,
                    _ => 0.0,
                };
            // Bent towards the opposite side, with the rim bending most (the drop's lens).
            let bend = DROP_BEND * distance.min(1.0).powi(2);
            let (gx, gy) = (0.5 - bend * dx, 0.5 - bend * dy);
            let at = (py * size + px) * 4;
            rgba[at..at + 4].copy_from_slice(&[
                255,
                (gx.clamp(0.0, 1.0) * 255.0).round() as u8,
                (gy.clamp(0.0, 1.0) * 255.0).round() as u8,
                (body.clamp(0.0, 1.0) * 255.0).round() as u8,
            ]);
        }
    }
    Ok(Picture { width: DROP_SIZE, height: DROP_SIZE, rgba })
}

/// Read the pictures and write the rain splash and wet road variants under `out/files/`, and the rain drops' replacement set under
/// `out/replace/`. `rain_splash`, `rain_drops` and `wet_road` are the mod's settings of those names. A failure only costs the
/// variants.
pub fn write(source: &Source, out: &Path, rain_splash: bool, rain_drops: bool, wet_road: bool) {
    let started = std::time::Instant::now();
    if wet_road {
        let result = write_text(out, WET_ROAD_FOLDER, "profile.toml", wet_road_profile_text());
        result.unwrap_or_else(|error| log::warn!("the wet road variant is not written: {error:#}"));
    }
    if !rain_splash && !rain_drops {
        return;
    }
    let (pictures, read) = match find(source) {
        Ok(found) => found,
        Err(error) => {
            log::warn!("the rain variants are not written: {error:#}");
            return;
        }
    };
    if rain_splash {
        let result = write_splash(out, &pictures);
        result.unwrap_or_else(|error| log::warn!("the rain splash variant is not written: {error:#}"));
    }
    if rain_drops {
        let result = write_drops(out, &pictures);
        result.unwrap_or_else(|error| log::warn!("the rain drops variant is not written: {error:#}"));
    }
    log::info!("rain variants: read {} KiB of the effects bundle, {} ms", read / 1024, started.elapsed().as_millis());
}

fn write_text(out: &Path, folder: &str, name: &str, text: &str) -> Result<()> {
    let folder = out.join("files").join(folder);
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    std::fs::write(folder.join(name), text).with_context(|| format!("writing {name}"))
}

fn write_splash(out: &Path, pictures: &RainPictures) -> Result<()> {
    let folder = out.join("files").join(SPLASH_FOLDER);
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    for (name, picture) in [("RING.PNG", &pictures.ring), ("SPRAY.PNG", &pictures.spray)] {
        std::fs::write(folder.join(name), vfx::png_bytes(picture)?).with_context(|| format!("writing {name}"))?;
    }
    write_text(out, SPLASH_FOLDER, "profile.toml", &splash_profile_text())
}

fn write_drops(out: &Path, pictures: &RainPictures) -> Result<()> {
    let folder = out.join("files").join(DROPS_FOLDER);
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    std::fs::write(folder.join(format!("{LENS_TARGET}.png")), vfx::png_bytes(&pictures.drop)?)
        .context("writing the lens drop")?;
    // The set is written last: a host that reads it finds the picture in place.
    let replace = out.join(blackbox_modding::content::REPLACE_FOLDER);
    std::fs::create_dir_all(&replace).with_context(|| format!("creating {}", replace.display()))?;
    std::fs::write(replace.join("rain-drops-mw2012.toml"), drops_manifest()).context("writing the replacement set")
}

#[cfg(test)]
#[path = "rain_assets_tests.rs"]
mod tests;
