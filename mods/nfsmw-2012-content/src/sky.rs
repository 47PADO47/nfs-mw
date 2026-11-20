//! The sky of the 2012 game as a value of the gameplay setting **Sky**: the cloud domes of its times of day and weather, and its moon.
//!
//! `HAWAII/ENVIRONMENT.BNDL` holds 25 timelines of keyframes; each names the cloud dome (an equirectangular picture of the upper
//! hemisphere, 2048 by 512 or 256, colour and cloud cover in the alpha) the sky shows from an hour, and two colours it lights it
//! with (`docs/formats/chameleon-environment.md`). This reads six domes and the moon, writes them as DDS pictures (the install's
//! compressed bytes, or halved when `sky_size` asks) with a profile of three weathers, and declares the value through a replacement
//! set of kind `sky` (`docs/modding/api-replace.md`). The game draws them as the clouds of its procedural sky
//! (`docs/modding/sky-variants.md`).
//!
//! Only the bundle's table and the blocks of the seven textures are read from the 5 MB file (a range read, like the trees); about
//! 5.5 MB is written under `files/SKY/MW2012/`. The setting `sky` (default `true`) turns it off.

mod plan;
mod profile;

use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use chameleon_gfx::{PixelFormat, Texture};

use crate::config::Config;
use crate::source::Source;
use crate::trees::ranges;
use plan::{DOMES, DomeSpec};

/// The id of the variant in the `sky_variant` option.
pub const VARIANT: &str = "mw2012";
/// The folder of the variant under the mod's `files/`, and the moon's file in it.
pub const FOLDER: &str = "SKY/MW2012";
pub const MOON_FILE: &str = "MOON.DDS";
/// The messages of the mod's catalogue (`lang/en`) for the option's label and the value's.
pub const LABEL_ID: &str = "nfsmw2012-sky";
pub const VALUE_LABEL_ID: &str = "nfsmw2012-sky-value";

/// The file of the dome called `name`.
pub fn dome_file(name: &str) -> String {
    format!("{}.DDS", name.to_ascii_uppercase())
}

/// Read the sky of the install and write the variant under `out/files/SKY/MW2012/` and `out/replace/`. A failure only costs the variant:
/// the game's own sky stays the only choice. Does nothing when the setting `sky` is off.
pub fn write(source: &Source, config: &Config, out: &Path) {
    if !config.sky {
        return;
    }
    if let Err(error) = try_write(source, config, out) {
        log::warn!("the sky variant is not written: {error:#}");
    }
}

fn try_write(source: &Source, config: &Config, out: &Path) -> Result<()> {
    let started = Instant::now();
    let path = source.root().join(chameleon_environment::BUNDLE_PATH);
    let (bundle, reader) = ranges::open(&path)?;
    let timelines = chameleon_environment::timelines(&bundle).context("reading the timelines")?;
    let weathers = plan::weathers(&timelines)?;
    let folder = out.join("files").join(FOLDER);
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    let mut written = 0;
    let mut domes: Vec<DomeSpec> = Vec::new();
    for dome in DOMES {
        let bytes = chameleon_environment::texture(&bundle, dome.key)
            .map_err(anyhow::Error::from)
            .and_then(|texture| picture(&texture, config.sky_size));
        match bytes {
            Ok(bytes) => {
                written += bytes.len();
                std::fs::write(folder.join(dome_file(dome.name)), bytes)
                    .with_context(|| format!("writing {}", dome.name))?;
                domes.push(dome);
            }
            Err(error) => log::warn!("sky variant: the dome {} ({:X}) is left out: {error:#}", dome.name, dome.key),
        }
    }
    let moon = match chameleon_environment::moon_texture(&bundle)
        .and_then(|key| chameleon_environment::texture(&bundle, key))
        .map_err(anyhow::Error::from)
        .and_then(|texture| texture.to_dds().map_err(anyhow::Error::from))
    {
        Ok(bytes) => {
            written += bytes.len();
            std::fs::write(folder.join(MOON_FILE), bytes).context("writing the moon")?;
            true
        }
        Err(error) => {
            log::warn!("sky variant: the moon is left out: {error:#}");
            false
        }
    };
    // A weather whose key names a dome that was left out cannot be used.
    let usable: Vec<_> = weathers
        .into_iter()
        .filter(|weather| weather.keys.iter().all(|key| domes.iter().any(|d| d.name == key.dome)))
        .collect();
    if usable.is_empty() || !moon {
        bail!("not enough of the sky could be read ({} weathers, moon {moon})", usable.len());
    }
    std::fs::write(folder.join("profile.toml"), profile::profile_text(&usable, &domes, moon).as_bytes())
        .context("writing the profile")?;
    // Written last: a host that reads the set finds the files in place.
    let replace_folder = out.join(blackbox_modding::content::REPLACE_FOLDER);
    std::fs::create_dir_all(&replace_folder).with_context(|| format!("creating {}", replace_folder.display()))?;
    std::fs::write(replace_folder.join("sky-mw2012.toml"), profile::replace_manifest(moon).as_bytes())
        .context("writing the replacement set")?;
    log::info!(
        "sky variant: {} domes, {} weathers, read {} KiB of the {} KiB bundle, wrote {} KiB, {} ms",
        domes.len(),
        usable.len(),
        reader.bytes_read() / 1024,
        std::fs::metadata(&path).map_or(0, |meta| meta.len() / 1024),
        written / 1024,
        started.elapsed().as_millis()
    );
    Ok(())
}

/// The DDS file of a dome: the install's bytes as they are when the picture is at most `max_width` wide, else halved (decoded,
/// averaged 2 by 2 and encoded again as DXT5) until it is.
fn picture(texture: &Texture, max_width: u32) -> Result<Vec<u8>> {
    if texture.width <= max_width {
        return Ok(texture.to_dds()?);
    }
    if texture.format != PixelFormat::Bc3 && texture.format != PixelFormat::Bc1 {
        bail!("only BC1 and BC3 pictures are resized");
    }
    let Some(rgba) = texture.to_rgba8() else { bail!("the picture does not decode") };
    let mut level = blackbox_tpk::Level { width: texture.width, height: texture.height, rgba };
    while level.width > max_width && level.width > 4 && level.height > 4 {
        level = blackbox_tpk::halve(&level);
    }
    let data = blackbox_tpk::encode_level(blackbox_tpk::PixelFormat::Dxt5, &level).context("encoding the picture")?;
    let halved = Texture {
        width: level.width,
        height: level.height,
        depth: 1,
        array_size: 1,
        mip_levels: 1,
        dxgi_format: 77,
        format: PixelFormat::Bc3,
        data,
    };
    Ok(halved.to_dds()?)
}

#[cfg(test)]
mod tests;
