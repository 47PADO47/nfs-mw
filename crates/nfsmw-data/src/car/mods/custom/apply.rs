//! Putting the selected choices of a mod car's own customization on its assembled model.

use std::collections::HashMap;

use anyhow::{Context, Result, bail, ensure};
use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use game_install::GameDir;
use glam::Mat4;

use super::{Customization, Kind, Paint as CustomPaint, Part, pick, vinyl_palette};
use crate::car::customization::{Category, Selection};
use crate::car::{CarModel, Paint, Placement, slot};

/// How many levels of detail a car has, `A` to `E`.
const LEVELS: usize = 5;

/// Apply `selection` to `model`, the assembled car in `folder`, drawn at level of detail `lod`. A selection that names
/// nothing keeps the car's default paint and no extra part.
pub fn apply(
    dir: &GameDir,
    folder: &str,
    custom: &Customization,
    selection: &Selection,
    lod: usize,
    model: &mut CarModel,
) -> Result<()> {
    let chosen = |category: Category| selection.parts[category as usize];
    let paint = chosen_paint(folder, custom, chosen(Category::Paint))?;
    set_paint(folder, paint, model);
    if let Some(hash) = chosen(Category::WindowTint) {
        let tint = pick(folder, Kind::Tint, &custom.tints, |t| &t.name, hash).context("the window tint is unknown")?;
        set_tint(tint.color, tint.opacity, model);
    }
    let models = [
        (Category::Wheels, Kind::Rim, &custom.rims),
        (Category::Spoiler, Kind::Spoiler, &custom.spoilers),
        (Category::Roof, Kind::RoofScoop, &custom.roof_scoops),
    ];
    let mut parts: Vec<(Category, &Part)> = Vec::new();
    for (category, kind, list) in models {
        let Some(hash) = chosen(category) else { continue };
        let part = pick(folder, kind, list, |p| &p.name, hash)
            .with_context(|| format!("the {} is unknown", category.label()))?;
        parts.push((category, part));
    }
    if !parts.is_empty() {
        let solids = crate::car::read_solids(dir, folder)?;
        let pack: HashMap<u32, Solid> = solids.into_iter().map(|solid| (solid.name_hash, solid)).collect();
        for (category, part) in parts {
            fit_part(category, part, &pack, lod, model)?;
        }
    }
    if let Some(hash) = chosen(Category::RimPaint) {
        let rim =
            pick(folder, Kind::RimPaint, &custom.rim_paints, |p| &p.name, hash).context("the rim paint is unknown")?;
        let base = model.light_materials.get(&bstring_hash("MAGSILVER")).cloned();
        let base = base.context("the shared rim material is missing")?;
        crate::car::rim_paint::recolour(model, base, rim.color);
    }
    let Some(hash) = chosen(Category::Vinyl) else {
        dress(dir, folder, custom, model);
        return Ok(());
    };
    // The shop lists liveries with the vinyls: the hash says which kind it is.
    match pick(folder, Kind::Livery, &custom.liveries, |l| &l.name, hash) {
        Some(livery) => super::livery::apply(dir, folder, livery, model)?,
        None => set_vinyl(dir, folder, custom, selection, hash, model)?,
    }
    Ok(())
}

/// The car wears the livery its manifest says it has from the start, when the player picked no vinyl. A picture that cannot be
/// drawn leaves the plain paint: a car is still a car.
fn dress(dir: &GameDir, folder: &str, custom: &Customization, model: &mut CarModel) {
    let Some(livery) = custom.default_livery.and_then(|position| custom.liveries.get(position)) else { return };
    if let Err(error) = super::livery::apply(dir, folder, livery, model) {
        log::warn!("{folder}: its livery {:?} is not drawn: {error:#}", livery.name);
    }
}

fn chosen_paint<'a>(folder: &str, custom: &'a Customization, hash: Option<u32>) -> Result<&'a CustomPaint> {
    let Some(hash) = hash else { return Ok(&custom.paints[custom.default_paint]) };
    pick(folder, Kind::Paint, &custom.paints, |p| &p.name, hash).context("the paint is unknown")
}

/// The body takes the paint: its colour and light material, and the flat texture the car's paint surfaces draw.
fn set_paint(folder: &str, paint: &CustomPaint, model: &mut CarModel) {
    let host = Paint {
        name: paint.name.clone(),
        rgb: paint.color,
        gloss: 128,
        light_material: Some(bstring_hash(paint.finish.light_material())),
    };
    let mut names: Vec<String> = model
        .textures
        .values()
        .filter(|texture| texture.name.ends_with("_SKIN1") || texture.name == "ROOF_SKIN")
        .map(|texture| texture.name.clone())
        .collect();
    names.push(format!("{}_SKIN1", folder.to_ascii_uppercase()));
    names.sort();
    names.dedup();
    for name in names {
        let texture = host.texture(&name);
        model.textures.insert(texture.name_hash, texture);
    }
    model.paint = Some(host);
    model.paintable = true;
}

/// The glass takes a copy of the windshield material, tinted and made as opaque as the tint says.
fn set_tint(rgb: [u8; 3], opacity: f32, model: &mut CarModel) {
    let Some(mut material) = model.light_materials.get(&bstring_hash("WINDSHIELD")).cloned() else { return };
    let scale = rgb.map(|v| f32::from(v) / 255.0);
    for (at, c) in scale.into_iter().enumerate() {
        material.diffuse_min[at] *= c;
        material.diffuse_max[at] *= c;
    }
    material.diffuse_min_alpha = opacity;
    material.diffuse_max_alpha = opacity;
    let hash = bstring_hash("__GARAGE_WINDOW_TINT");
    material.name = "__GARAGE_WINDOW_TINT".to_owned();
    material.name_hash = hash;
    model.light_materials.insert(hash, material);
    model.window_tint_material = Some(hash);
}

/// The solid of `base` at `level`, or the nearest level the car's pack has, added to the model's solids.
fn add_model(model: &mut CarModel, pack: &HashMap<u32, Solid>, base: &str, level: usize) -> Option<u32> {
    let order = (0..=level).rev().chain(level + 1..LEVELS);
    for at in order {
        let hash = bstring_hash(&format!("{base}_{}", char::from(b'A' + at as u8)));
        let Some(solid) = pack.get(&hash) else { continue };
        model.solids.entry(hash).or_insert_with(|| solid.clone());
        return Some(hash);
    }
    None
}

/// The levels the model is drawn at, with the placements of each: the canonical level and the driving levels.
fn lists(model: &mut CarModel, lod: usize) -> Vec<(usize, &mut Vec<Placement>)> {
    let mut all = vec![(lod, &mut model.placements)];
    all.extend(model.driving_lods.levels.iter_mut().map(|l| (l.level, &mut l.placements)));
    all
}

/// Rims replace the wheel solids; a spoiler or a roof scoop is a solid in car space added to the car.
fn fit_part(
    category: Category,
    part: &Part,
    pack: &HashMap<u32, Solid>,
    lod: usize,
    model: &mut CarModel,
) -> Result<()> {
    let levels: Vec<usize> = lists(model, lod).iter().map(|(level, _)| *level).collect();
    let mut found = HashMap::new();
    for level in levels {
        if let Some(hash) = add_model(model, pack, &part.model, level) {
            found.insert(level, hash);
        }
    }
    ensure!(found.contains_key(&lod), "the {}: the model {} has no solid", category.label(), part.model);
    let target = match category {
        Category::Wheels => None,
        Category::Spoiler => Some(slot::SPOILER),
        Category::Roof => Some(slot::ROOF),
        _ => bail!("{} is not a model", category.label()),
    };
    for (level, placements) in lists(model, lod) {
        let Some(&solid) = found.get(&level) else { continue };
        let Some(mount) = target else {
            for placement in placements.iter_mut().filter(|p| [slot::FRONT_WHEEL, slot::REAR_WHEEL].contains(&p.slot)) {
                placement.solid = solid;
            }
            continue;
        };
        placements.retain(|p| p.slot != mount);
        placements.push(Placement { solid, transform: Mat4::IDENTITY, left_brake: false, slot: mount, corner: None });
    }
    Ok(())
}

/// The vinyl drawn over the paint: its pictures from the car's own `VINYLS.BIN`, weighed into the channel colours.
fn set_vinyl(
    dir: &GameDir,
    folder: &str,
    custom: &Customization,
    selection: &Selection,
    hash: u32,
    model: &mut CarModel,
) -> Result<()> {
    let vinyl = pick(folder, Kind::Vinyl, &custom.vinyls, |v| &v.name, hash).context("the vinyl is unknown")?;
    let wanted = [bstring_hash(&vinyl.bitmap), bstring_hash(&vinyl.mask)];
    let found = crate::car::vinyl::textures(dir, folder, &wanted.into_iter().collect())?;
    let bitmap = found.get(&wanted[0]).context("the vinyl bitmap is missing")?;
    let mask = found.get(&wanted[1]).context("the vinyl mask is missing")?;
    let palette = vinyl_palette(custom);
    let colors: [[u8; 3]; 3] = std::array::from_fn(|channel| {
        selection.vinyl_colors[channel].unwrap_or_else(|| palette.get(channel).map_or([255; 3], |(_, rgb)| *rgb))
    });
    let paint = model.paint.as_ref().context("a vinyl needs the body paint")?;
    let template = paint.texture(crate::car::vinyl::LAYER);
    let layer = crate::car::vinyl::compose(bitmap, mask, usize::from(vinyl.channels), colors, template)?;
    model.textures.insert(layer.name_hash, layer);
    crate::car::vinyl::repaint(model)
}
