//! Which bundles become one car, and the livery pictures a car offers: [`liveries`] decides on the vehicle list and the
//! shape of the cars; this reads the bundles that need it.

use chameleon_gfx::Texture;
use rayon::prelude::*;

use crate::liveries::{self, Group, LiveryChoice};
use crate::source::{CarFile, Source, SourceCar};
use crate::textures;

/// A car to convert, and the bundles whose pictures it offers as liveries.
pub struct Planned<'a> {
    pub file: &'a CarFile,
    pub liveries: Vec<&'a CarFile>,
}

/// Every car of the install: one per vehicle of the vehicle list, with the bundles of its liveries. A livery bundle that
/// does not match its base (another body, or no picture to take) is a car of its own, as are the bundles the list does
/// not mention, and so is every bundle when the install's list does not read.
pub fn plan(source: &Source) -> Vec<Planned<'_>> {
    let numbers: Vec<u32> = source.cars().iter().map(|car| car.number).collect();
    let groups = liveries::groups(source.list(), &numbers);
    let planned: Vec<Vec<Planned>> = groups.par_iter().map(|group| plan_group(source, group)).collect();
    planned.into_iter().flatten().collect()
}

fn plan_group<'a>(source: &'a Source, group: &Group) -> Vec<Planned<'a>> {
    let find = |number: u32| source.cars().iter().find(|car| car.number == number);
    let Some(base) = find(group.base) else { return Vec::new() };
    let candidates: Vec<&CarFile> = group.liveries.iter().filter_map(|&number| find(number)).collect();
    let (same, apart) = split(source, base, candidates);
    let mut out = vec![Planned { file: base, liveries: same }];
    out.extend(apart.into_iter().map(|file| Planned { file, liveries: Vec::new() }));
    out
}

/// The candidates that are the car of `base` in another picture, and the ones that are not.
fn split<'a>(source: &Source, base: &CarFile, candidates: Vec<&'a CarFile>) -> (Vec<&'a CarFile>, Vec<&'a CarFile>) {
    if candidates.is_empty() {
        return (Vec::new(), candidates);
    }
    let base_view = match source.livery_view(base) {
        Ok(view) => view,
        Err(error) => {
            log::warn!("{}: the liveries are converted as cars: {error:#}", base.path.display());
            return (Vec::new(), candidates);
        }
    };
    candidates.into_iter().partition(|file| match source.livery_view(file) {
        Ok(view) => {
            let same = liveries::same_car(&base_view.signature, &view.signature);
            if !same {
                log::warn!("{}: not the car of {}: converted as a car", file.path.display(), base.path.display());
            }
            same
        }
        Err(error) => {
            log::warn!("{}: the livery does not read: {error:#}", file.path.display());
            false
        }
    })
}

/// The liveries of the car `car` in `folder`: the picture of its own bundle first, then those of `files`, each with the
/// choice that names it. A car with no `files` offers none (its own picture alone is not a choice). A bundle whose
/// picture cannot be carried over is left out, and the others keep their numbers.
pub fn pictures(source: &Source, car: &SourceCar, folder: &str, files: &[&CarFile]) -> Vec<(LiveryChoice, Texture)> {
    if files.is_empty() {
        return Vec::new();
    }
    let own = liveries::signature(&car.vehicle).picture.and_then(|id| car.textures.get(&id).cloned());
    let mut all = vec![own];
    for file in files {
        all.push(source.livery_view(file).ok().and_then(|view| view.picture));
    }
    let mut out = Vec::new();
    for (position, picture) in all.into_iter().enumerate() {
        let choice = liveries::choice(folder, position);
        match picture {
            Some(picture) if textures::can_carry(&picture) => out.push((choice, picture)),
            _ => log::warn!("{folder}: {} has no picture that can be carried over: left out", choice.name),
        }
    }
    out
}
