//! Which bundles become one car, and the livery pictures a car offers: [`liveries`] decides on the vehicle list and the
//! shape of the cars, [`crate::fold`] on the vehicles that are one car under two ids; this reads the bundles that need it.

use chameleon_gfx::Texture;
use rayon::prelude::*;

use crate::fold::{self, Folded};
use crate::liveries::{self, LiveryChoice};
use crate::source::{CarFile, Source, SourceCar};
use crate::textures;

/// A car to convert, and the bundles whose pictures it offers as liveries.
pub struct Planned<'a> {
    pub file: &'a CarFile,
    pub liveries: Vec<&'a CarFile>,
    /// The bundles of the other vehicles that are this car under another vehicle id: the car is offered their paints too.
    pub also: Vec<&'a CarFile>,
}

/// Every car of the install: one per vehicle of the vehicle list, with the bundles of its liveries, and one per car that the
/// list has under two vehicle ids ([`fold::fold_same_car`], by the `name` of the car in a bundle, `None` for one that is never
/// folded). A livery bundle that does not match its base (another body, or no picture to take) is a car of its own, as are the
/// bundles the list does not mention, and so is every bundle when the install's list does not read.
pub fn plan<'a>(source: &'a Source, name: &dyn Fn(u32) -> Option<String>) -> Vec<Planned<'a>> {
    let numbers: Vec<u32> = source.cars().iter().map(|car| car.number).collect();
    let groups = liveries::groups(source.list(), &numbers);
    let colours = |number: u32| {
        source.cars().iter().find(|car| car.number == number).map_or(0, |car| source.paints_of(car).len())
    };
    let folded = fold::fold_same_car(groups, name, &colours);
    let planned: Vec<Vec<Planned>> = folded.par_iter().map(|group| plan_group(source, group)).collect();
    planned.into_iter().flatten().collect()
}

fn plan_group<'a>(source: &'a Source, folded: &Folded) -> Vec<Planned<'a>> {
    let find = |number: u32| source.cars().iter().find(|car| car.number == number);
    let Some(base) = find(folded.group.base) else { return Vec::new() };
    let candidates: Vec<&CarFile> = folded.group.liveries.iter().filter_map(|&number| find(number)).collect();
    let (same, apart) = split(source, base, candidates);
    let also = folded.also.iter().filter_map(|&number| find(number)).collect();
    let mut out = vec![Planned { file: base, liveries: same, also }];
    out.extend(apart.into_iter().map(|file| Planned { file, liveries: Vec::new(), also: Vec::new() }));
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

/// The names the shop gives the liveries of the car whose own bundle is `base` and whose other liveries are in `files`, by
/// position (the own bundle first): the game's names where its paint packs give them (`Hero`, `Race`), else `None`.
pub fn livery_names(source: &Source, base: &CarFile, files: &[&CarFile]) -> Vec<Option<String>> {
    let all: Vec<&CarFile> = std::iter::once(base).chain(files.iter().copied()).collect();
    liveries::display_names(&source.livery_names(&all))
}

/// The liveries of the car `car` in `folder`: the picture of its own bundle first, then those of `files`, each with the
/// choice that names it (`names` has the name of each by position, see [`livery_names`]). A car whose own picture is a
/// livery ([`liveries::own_picture`]) offers it even with no `files`: it is how the 2012 game dresses the car (a police car's
/// markings, the Porsche 918 Spyder's stripes). A bundle whose picture cannot be carried over is left out, and the others
/// keep their numbers.
pub fn pictures(
    source: &Source,
    car: &SourceCar,
    folder: &str,
    files: &[&CarFile],
    names: &[Option<String>],
) -> Vec<(LiveryChoice, Texture)> {
    let own = liveries::own_picture(&car.vehicle).and_then(|id| car.textures.get(&id).cloned());
    if files.is_empty() && own.is_none() {
        return Vec::new();
    }
    let mut all = vec![own];
    for file in files {
        all.push(source.livery_view(file).ok().and_then(|view| view.picture));
    }
    let mut out = Vec::new();
    for (position, picture) in all.into_iter().enumerate() {
        let name = names.get(position).and_then(|name| name.as_deref());
        let choice = liveries::named_choice(folder, position, name);
        match picture {
            Some(picture) if textures::can_carry(&picture) => out.push((choice, picture)),
            _ => log::warn!("{folder}: {} has no picture that can be carried over: left out", choice.name),
        }
    }
    out
}
