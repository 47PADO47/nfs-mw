//! The garage catalogue of a mod car, from its own `[customization]` table and its own files.

use std::collections::HashSet;

use anyhow::Result;
use blackbox_hash::bstring_hash;
use blackbox_tpk::decode_rgba8;
use game_install::GameDir;

use super::{Customization, Kind, part_hash, vinyl_palette};
use crate::car::customization::{Catalog, Category, Choice, ColorChoice};

fn choice(hash: u32, label: &str) -> Choice {
    Choice { hash, label: label.to_owned(), language_hash: None, carbon: false }
}

/// The catalogue of the car in `folder`. A model the car's `GEOMETRY.BIN` lacks, a vinyl whose pictures its
/// `VINYLS.BIN` lacks or cannot decode, or a livery whose picture its `TEXTURES.BIN` lacks, is not offered.
pub fn catalog(dir: &GameDir, folder: &str, custom: &Customization) -> Result<Catalog> {
    let mut catalog = Catalog::default();
    paints(folder, custom, &mut catalog);
    for rim in &custom.rim_paints {
        let hash = part_hash(folder, Kind::RimPaint, &rim.name);
        catalog.colors.insert(hash, rim.color);
        catalog.choices[Category::RimPaint as usize].push(choice(hash, &rim.name));
    }
    catalog.custom_rim_paint = !custom.rim_paints.is_empty();
    for tint in &custom.tints {
        let hash = part_hash(folder, Kind::Tint, &tint.name);
        catalog.choices[Category::WindowTint as usize].push(choice(hash, &tint.name));
    }
    let models = models(dir, folder, custom)?;
    for (kind, category, parts) in [
        (Kind::Rim, Category::Wheels, &custom.rims),
        (Kind::Spoiler, Category::Spoiler, &custom.spoilers),
        (Kind::RoofScoop, Category::Roof, &custom.roof_scoops),
    ] {
        for part in parts.iter().filter(|part| models.contains(&bstring_hash(&format!("{}_A", part.model)))) {
            let hash = part_hash(folder, kind, &part.name);
            catalog.choices[category as usize].push(choice(hash, &part.name));
            if kind == Kind::Rim {
                catalog.paintable_wheels.insert(hash);
                catalog.rim_metadata.insert(hash, (0, 0));
            }
        }
    }
    vinyls(dir, folder, custom, &mut catalog)?;
    super::livery::offer(dir, folder, custom, &mut catalog)?;
    if let Some(plate) = &custom.plate {
        catalog.plate = true;
        catalog.plate_designs = super::plate_design::offered(dir, folder, plate);
    }
    for list in [Category::Wheels, Category::Spoiler, Category::Roof, Category::WindowTint, Category::Vinyl] {
        for entry in &catalog.choices[list as usize] {
            catalog.part_groups.insert(entry.hash, 0);
        }
    }
    Ok(catalog)
}

/// The body paints: the default one is the car's stock paint, so it is not among the choices.
pub(super) fn paints(folder: &str, custom: &Customization, catalog: &mut Catalog) {
    let stock = part_hash(folder, Kind::Paint, &custom.paints[custom.default_paint].name);
    catalog.stock[Category::Paint as usize] = Some(stock);
    for (at, paint) in custom.paints.iter().enumerate() {
        let hash = part_hash(folder, Kind::Paint, &paint.name);
        catalog.colors.insert(hash, paint.color);
        catalog.paint_families.insert(hash, paint.finish.family().to_owned());
        if at == custom.default_paint {
            continue;
        }
        catalog.choices[Category::Paint as usize].push(choice(hash, &paint.name));
        if !catalog.finishes.iter().any(|(label, _)| label == paint.finish.label()) {
            catalog.finishes.push((paint.finish.label().to_owned(), hash));
        }
    }
}

/// The name hashes of every solid in the car's `GEOMETRY.BIN`, when the car lists a model at all.
fn models(dir: &GameDir, folder: &str, custom: &Customization) -> Result<HashSet<u32>> {
    let listed = !(custom.rims.is_empty() && custom.spoilers.is_empty() && custom.roof_scoops.is_empty());
    if !listed {
        return Ok(HashSet::new());
    }
    Ok(crate::car::read_solids(dir, folder)?.into_iter().map(|solid| solid.name_hash).collect())
}

fn vinyls(dir: &GameDir, folder: &str, custom: &Customization, catalog: &mut Catalog) -> Result<()> {
    if custom.vinyls.is_empty() {
        return Ok(());
    }
    let wanted: HashSet<u32> =
        custom.vinyls.iter().flat_map(|v| [bstring_hash(&v.bitmap), bstring_hash(&v.mask)]).collect();
    let found = crate::car::vinyl::textures(dir, folder, &wanted)?;
    for vinyl in &custom.vinyls {
        let pair = found.get(&bstring_hash(&vinyl.bitmap)).zip(found.get(&bstring_hash(&vinyl.mask)));
        let usable = pair.is_some_and(|(bitmap, mask)| {
            bitmap.width == mask.width
                && bitmap.height == mask.height
                && decode_rgba8(bitmap).is_some()
                && decode_rgba8(mask).is_some()
        });
        if !usable {
            log::warn!(
                "{folder}: the vinyl {:?} is not offered: its pictures are missing or cannot be read",
                vinyl.name
            );
            continue;
        }
        let hash = part_hash(folder, Kind::Vinyl, &vinyl.name);
        catalog.choices[Category::Vinyl as usize].push(choice(hash, &vinyl.name));
        catalog.vinyl_remaps.insert(hash, usize::from(vinyl.channels));
    }
    let palette: Vec<ColorChoice> = vinyl_palette(custom)
        .into_iter()
        .map(|(name, rgb)| ColorChoice { hash: part_hash(folder, Kind::VinylColor, &name), rgb, upgrade_level: 0 })
        .collect();
    for channel in catalog.color_palettes.iter_mut().take(3) {
        *channel = palette.clone();
    }
    Ok(())
}
