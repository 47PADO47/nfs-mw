//! The colour grading of the 2012 game as a value of the `grading_variant` option (`GRADING/MW2012/` in the mod's `files/`, the
//! value **NFS Most Wanted 2012** of the Video setting "Color Grading"). Spec of the profile:
//! `docs/modding/effect-variants.md`, "Profile format of the grading"; what the 2012 data holds, which cube is which, and the
//! evidence: `docs/formats/chameleon-environment-grading.md`.
//!
//! `HAWAII/ENVIRONMENT.BNDL` holds 25 `EnvironmentTimeline` objects, each a list of hours of the day and the keyframe (lighting,
//! fog, sky, clouds, one colour cube) in force from that hour; every keyframe also names a wet keyframe with a cube of its
//! own. This reads the bundle by ranges (only its table and the small objects, never its textures), picks one timeline and
//! writes the cubes it names as `.LUT` files (12 KB each) and a profile of its hourly keys:
//!
//! - **clear**: the dry cube of each key of the first of the timelines that change cube through the day (the lowest resource
//!   id of those with several cubes): night, a pre-dawn, dawn, morning, noon and evening cubes at the hours the 2012 data gives;
//! - **rain**: the wet cube of each of the same keys (a wet night, dawn, morning, noon and evening; the pre-dawn key has no wet
//!   twin and keeps its own cube, which is already as cool as the wet night's). That the wet keyframe is
//!   the rain look is a reading **[unconfirmed]**: the cubes are the cool, teal-lifted twins of the dry ones.
//!
//! The other 24 timelines differ from the first mostly in their keyframes, not their cubes; one more (eight keys, a single
//! cube all day) is not used, its meaning being unknown.

use std::path::Path;

use anyhow::{Context, Result, bail};
use chameleon_postfx::environment::{Timeline, TimelineKey, cube_rgb, timelines};

use crate::source::Source;

/// The id of the variant in the `grading_variant` option, and what the menu shows for it.
pub const VARIANT: &str = "mw2012";
pub const LABEL: &str = "NFS Most Wanted 2012";
/// The folder of the variant under the mod's `files/`, and the bundle with the cubes.
pub const FOLDER: &str = "GRADING/MW2012";
pub const BUNDLE: &str = "HAWAII/ENVIRONMENT.BNDL";

/// The timeline a profile is made of: its dry cubes are the clear set, its wet cubes the rain set.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan<'a> {
    pub timeline: &'a Timeline,
}

/// The name a profile gives the cube of resource `id`: `c` and the low 24 bits in hex.
pub fn cube_name(id: u64) -> String {
    format!("c{:06x}", id & 0xff_ffff)
}

/// The file name of the cube of resource `id`.
pub fn file_name(id: u64) -> String {
    format!("C{:06X}.LUT", id & 0xff_ffff)
}

/// The timeline to use among `all`: the first that changes its cube through the day (see the module's description).
pub fn plan(all: &[Timeline]) -> Option<Plan<'_>> {
    let timeline = all.iter().filter(|t| t.cubes().len() >= 3).min_by_key(|t| t.id & 0xff_ffff_ffff)?;
    Some(Plan { timeline })
}

/// Whether any key of the plan's timeline has a wet cube (otherwise the profile has no rain set). A key without one (the
/// pre-dawn keyframe of the first two timelines has none) keeps its dry cube in the rain set.
pub fn has_rain(plan: &Plan<'_>) -> bool {
    plan.timeline.keys.iter().any(|key| key.wet_cube.is_some())
}

/// The cubes of `plan` in order of first use: the dry ones, then the wet ones (when the plan has a rain set).
pub fn cubes(plan: &Plan<'_>) -> Vec<u64> {
    let mut found = plan.timeline.cubes();
    if !has_rain(plan) {
        return found;
    }
    for cube in plan.timeline.wet_cubes() {
        if !found.contains(&cube) {
            found.push(cube);
        }
    }
    found
}

/// One `[[set]]`: the keys of `timeline` with the cube `pick` gives each.
fn set_text(weather: &str, timeline: &Timeline, pick: impl Fn(&TimelineKey) -> Option<u64>) -> String {
    let mut text = format!("\n[[set]]\nweather = \"{weather}\"\n");
    for key in &timeline.keys {
        let Some(cube) = pick(key) else { continue };
        text.push_str(&format!("[[set.key]]\nhour = {:?}\ncube = \"{}\"\n", key.hour, cube_name(cube)));
    }
    text
}

/// The text of the profile of `plan`.
pub fn profile_text(plan: &Plan<'_>) -> String {
    let mut text = String::from(
        "# The colour grading of NFS Most Wanted (2012): the cubes of HAWAII/ENVIRONMENT.BNDL by hour of the day.\nformat = 1\n",
    );
    for cube in cubes(plan) {
        text.push_str(&format!(
            "\n[[cube]]\nname = \"{}\"\nfile = \"{FOLDER}/{}\"\n",
            cube_name(cube),
            file_name(cube)
        ));
    }
    text.push_str(&set_text("clear", plan.timeline, |key| Some(key.cube)));
    if has_rain(plan) {
        text.push_str(&set_text("rain", plan.timeline, |key| key.wet_cube.or(Some(key.cube))));
    }
    text
}

/// Read the cubes of the install and write the variant under `out/files/GRADING/MW2012/`. A failure only costs the variant: the
/// picture stays as the game draws it.
pub fn write(source: &Source, out: &Path) {
    if let Err(error) = try_write(source, out) {
        log::warn!("the colour grading variant is not written: {error:#}");
    }
}

fn try_write(source: &Source, out: &Path) -> Result<()> {
    let path = source.root().join(BUNDLE);
    let (bundle, ranges) = crate::trees::ranges::open(&path)?;
    let all = timelines(&bundle);
    let Some(plan) = plan(&all) else { bail!("{BUNDLE} has no timeline that changes its cube through the day") };
    let folder = out.join("files").join(FOLDER);
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    let wanted = cubes(&plan);
    for cube in &wanted {
        let Some(data) = cube_rgb(&bundle, *cube) else { bail!("the cube {} does not read", cube_name(*cube)) };
        let bytes = blackbox_modding::content::encode_lut(data.size, &data.rgb);
        std::fs::write(folder.join(file_name(*cube)), bytes)
            .with_context(|| format!("writing {}", file_name(*cube)))?;
    }
    std::fs::write(folder.join("profile.toml"), profile_text(&plan)).context("writing the grading profile")?;
    log::info!(
        "colour grading variant: {} cubes, {} keys, rain set {} ({} bytes read of {BUNDLE})",
        wanted.len(),
        plan.timeline.keys.len(),
        has_rain(&plan),
        ranges.bytes_read()
    );
    Ok(())
}

#[cfg(test)]
mod tests;
