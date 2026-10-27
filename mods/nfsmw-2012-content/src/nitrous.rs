//! The nitrous of the 2012 game as a variant of the 2005 game's: its sounds, found in the heat level sound set, written as
//! WAV files with a `profile.toml` that the game plays (`docs/modding/nitrous-variants.md`, evidence in
//! `docs/formats/chameleon-nitrous.md`).
//!
//! The 2012 game plays the nitrous from two sound sequences (kind `0x15` objects) of `GAMELOGIC/HL_*.BNDL`. The sequence
//! that reads `NitrousState.UsingNitrous` plays a looping sample over the burn and a one-shot after it; the sequence that
//! reads only the `NITROUS_POSTFX_OFF` trigger plays one longer one-shot. The sequences are found by those expression strings
//! and the samples by their imports, so no resource id of the 2012 game is written here. Which sample plays *when* is read
//! from the loop flag of the stream (the loop is the burn) and from the sequence it sits in; the timing inside a sequence is
//! not decoded, so the profile's fades are the game's defaults, not the 2012 game's.

use std::path::Path;

use anyhow::{Context, Result};
use chameleon_bundle::{Bundle, ResourceKind};

use crate::sample::{self, SAMPLE_KIND, StereoSample};
use crate::source::Source;
use crate::vfx::{self, GLOBAL_EFFECTS};
use crate::vfx_profile::{self, EffectSet};

/// The id of the variant in the `nitrous_variant` option, and the folder name of its files.
pub const VARIANT: &str = "mw2012";
/// What the menu shows for it.
pub const LABEL: &str = "NFS Most Wanted 2012";

const BURN_EXPRESSION: &[u8] = b"NitrousState.UsingNitrous";
const TRIGGER_EXPRESSION: &[u8] = b"NITROUS_POSTFX_OFF";

/// The three sounds of the nitrous, stereo.
#[derive(Debug, Clone, Default)]
pub struct Found {
    /// Played once as the nitrous lights.
    pub start: Option<StereoSample>,
    /// Played over the burn, looping.
    pub looped: Option<StereoSample>,
    /// Played once as it stops.
    pub end: Option<StereoSample>,
    /// The particle effects and their pictures, when the install gives them.
    pub effects: Option<EffectSet>,
}

impl Found {
    pub fn is_empty(&self) -> bool {
        !self.has_sounds() && self.effects.is_none()
    }

    fn has_sounds(&self) -> bool {
        self.start.is_some() || self.looped.is_some() || self.end.is_some()
    }
}

/// The nitrous sounds of one bundle, when it holds the sequences.
pub fn find(bundle: &Bundle) -> Option<Found> {
    let kind = ResourceKind::GenesysObject.id();
    let (mut burn, mut ignite) = (Vec::new(), Vec::new());
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind || entry.import_count == 0 {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        let Some(main) = resource.blocks.first() else { continue };
        let reads_burn = contains(main, BURN_EXPRESSION);
        let reads_trigger = contains(main, TRIGGER_EXPRESSION);
        if !reads_burn && !reads_trigger {
            continue;
        }
        let samples: Vec<StereoSample> = resource
            .imports
            .iter()
            .filter_map(|import| bundle.find(import.id))
            .filter(|at| bundle.entries().get(*at).is_some_and(|entry| entry.kind == SAMPLE_KIND))
            .filter_map(|at| sample::read_stereo(bundle, at))
            .collect();
        if samples.is_empty() {
            continue;
        }
        match reads_burn {
            true => burn.push(samples),
            false => ignite.push(samples),
        }
    }
    let found = assign(burn.into_iter().next().unwrap_or_default(), ignite.into_iter().next().unwrap_or_default());
    (!found.is_empty()).then_some(found)
}

/// Which sample is which: in the sequence that follows the burn the looping one is the loop and a one-shot is the end; in
/// the other sequence the first one-shot is the start.
fn assign(burn: Vec<StereoSample>, ignite: Vec<StereoSample>) -> Found {
    let looped = burn.iter().find(|sample| sample.looped).cloned();
    let end = burn.iter().find(|sample| !sample.looped).cloned();
    let start = ignite.into_iter().find(|sample| !sample.looped);
    Found { start, looped, end, effects: None }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

/// The first of the install's logic bundles that has the nitrous sequences.
pub fn find_in(paths: &[std::path::PathBuf]) -> Option<Found> {
    for path in paths {
        let Ok(data) = std::fs::read(path) else { continue };
        let Ok(bundle) = Bundle::parse(&data) else { continue };
        if let Some(found) = find(&bundle) {
            return Some(found);
        }
    }
    None
}

/// The `profile.toml` for the sounds that were found: only the sounds it has are named. The fades are the game's defaults,
/// not read from the 2012 data.
pub fn profile_text(found: &Found) -> String {
    let mut text = String::from("format = 1\n");
    let folder = format!("NITROUS/{}", VARIANT.to_ascii_uppercase());
    if found.has_sounds() {
        text.push_str("\n[audio]\n");
    }
    for (key, sample, name) in
        [("start", &found.start, "START"), ("loop", &found.looped, "LOOP"), ("end", &found.end, "END")]
    {
        if sample.is_some() {
            text.push_str(&format!("{key} = \"{folder}/{name}.WAV\"\n"));
        }
    }
    if found.has_sounds() {
        text.push_str("replace_original = true\n");
    }
    let Some(effects) = &found.effects else { return text };
    // The 2012 effects stand in for the game's blue nitrous flame and for its blow-off and backfire flame.
    text.push_str(&format!("\n[flame]\nreplace_burn = {}\nreplace_pop = {}\n", effects.has_burn, effects.has_pop));
    text.push_str(&effects.toml);
    text
}

/// The tail pipe effects of the 2012 game: found through the `SpotEffect` records of the first car that has the three, built
/// from the emitters of `GLOBALEFFECTS.BNDL` (`vfx`, `vfx_profile`). `None` when the install has none that read.
pub fn find_effects(source: &Source) -> Option<EffectSet> {
    let global = source.read_file(GLOBAL_EFFECTS).ok()?;
    let global = Bundle::parse(&global).ok()?;
    for file in source.cars() {
        let Ok(bytes) = source.bundle_bytes(file) else { continue };
        let Ok(car) = Bundle::parse(&bytes) else { continue };
        let Some(roles) = vfx::roles(&vfx::spots(&car)) else { continue };
        let emitters =
            |id| vfx::emitters(&global, id).map_err(|error| log::warn!("nitrous effect {id:#x}: {error:#}")).ok();
        let (start, burn, pop) = (emitters(roles.start)?, emitters(roles.burn)?, emitters(roles.pop)?);
        return vfx_profile::build(&start, &burn, &pop);
    }
    None
}

/// Write the sounds and the profile under `out/files/NITROUS/<VARIANT>/`.
pub fn write(out: &Path, found: &Found) -> Result<()> {
    let folder = out.join("files").join("NITROUS").join(VARIANT.to_ascii_uppercase());
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    for (name, sample) in [("START", &found.start), ("LOOP", &found.looped), ("END", &found.end)] {
        let Some(sample) = sample else { continue };
        let bytes = sample::wav_stereo_bytes(&sample.frames, sample.rate, 1.0);
        std::fs::write(folder.join(format!("{name}.WAV")), bytes).with_context(|| format!("writing {name}.WAV"))?;
    }
    for (name, picture) in found.effects.iter().flat_map(|effects| &effects.pictures) {
        let bytes = vfx::png_bytes(picture)?;
        std::fs::write(folder.join(name), bytes).with_context(|| format!("writing {name}"))?;
    }
    std::fs::write(folder.join("profile.toml"), profile_text(found)).context("writing the nitrous profile")
}

#[cfg(test)]
mod tests;
