//! Fixed-UV bitmap vinyls. No placement, scale or rotation editor is involved.
use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, ensure};
use blackbox_carparts::{CarTypeInfo, Part};
use blackbox_hash::bstring_hash;
use blackbox_tpk::{AlphaUsage, PixelFormat, Texture, decode_rgba8};
use game_install::GameDir;

use super::{CarModel, CarTables, Slots, slot};

pub(super) const LAYER: &str = "__GARAGE_VINYL";

fn names(t: &CarTables, car: &CarTypeInfo, part: &Part) -> (String, String) {
    let name = format!("{}_{}", car.base_model_name, t.parts.name(part));
    (name.clone(), format!("{name}_MASK"))
}

/// The vinyl artwork of `folder`, from the car's own `VINYLS.BIN` and `PREVINYL.BIN`. The artwork is drawn over the
/// paint texture, so it is authored in the layout of the car's own paint surfaces.
pub(super) fn textures(dir: &GameDir, folder: &str, wanted: &HashSet<u32>) -> Result<HashMap<u32, Texture>> {
    let mut found = HashMap::new();
    for file in ["VINYLS.BIN", "PREVINYL.BIN"] {
        let path = format!("CARS/{folder}/{file}");
        if !dir.exists(&path) {
            continue;
        }
        let packs =
            blackbox_tpk::read_texture_packs_filtered(&crate::read_unwrapped(dir, &path)?, |h| wanted.contains(&h))?;
        for texture in packs.into_iter().flat_map(|pack| pack.textures) {
            found.entry(texture.name_hash).or_insert(texture);
        }
    }
    Ok(found)
}

/// Only offer installed bitmap/mask pairs with supported color weights and matching dimensions.
pub(super) fn available(dir: &GameDir, folder: &str, t: &CarTables, car: &CarTypeInfo) -> Result<HashSet<u32>> {
    if !car.skinnable {
        return Ok(HashSet::new());
    }
    let parts: Vec<_> = t
        .parts
        .parts()
        .iter()
        .filter(|p| p.part_id == 79 && t.parts.attribute(p, "NUMREMAPCOLOURS").unwrap_or(0) <= 3)
        .collect();
    let wanted = parts
        .iter()
        .flat_map(|p| {
            let (a, b) = names(t, car, p);
            [bstring_hash(&a), bstring_hash(&b)]
        })
        .collect();
    let found = textures(dir, folder, &wanted)?;
    Ok(parts
        .into_iter()
        .filter(|p| {
            let (a, b) = names(t, car, p);
            let pair = found.get(&bstring_hash(&a)).zip(found.get(&bstring_hash(&b)));
            pair.is_some_and(|(a, b)| {
                a.width == b.width && a.height == b.height && decode_rgba8(a).is_some() && decode_rgba8(b).is_some()
            })
        })
        .map(|p| p.name_hash)
        .collect())
}

pub(super) fn apply(
    dir: &GameDir,
    folder: &str,
    t: &CarTables,
    car: &CarTypeInfo,
    slots: &Slots,
    selection: &super::customization::Selection,
    model: &mut CarModel,
) -> Result<()> {
    let Some(part) = slots[slot::VINYL_LAYER] else {
        return Ok(());
    };
    ensure!(model.paintable, "this car has a fixed livery");
    let (name, mask) = names(t, car, &part);
    let found = textures(dir, folder, &HashSet::from([bstring_hash(&name), bstring_hash(&mask)]))?;
    let texture = found.get(&bstring_hash(&name)).context("vinyl bitmap is missing")?;
    let mask = found.get(&bstring_hash(&mask)).context("vinyl mask is missing")?;
    let colors: [[u8; 3]; 3] = std::array::from_fn(|i| {
        selection.vinyl_colors[i]
            .unwrap_or_else(|| slots[79 + i].map(|p| super::Paint::of(t, &p).rgb).unwrap_or([255; 3]))
    });
    let remap = t.parts.attribute(&part, "NUMREMAPCOLOURS").unwrap_or(0) as usize;
    let layer = compose(texture, mask, remap, colors, super::Paint::of(t, &part).texture(LAYER))?;
    model.textures.insert(layer.name_hash, layer);
    repaint(model)
}

/// The vinyl layer: the bitmap weighed into `remap` colour channels (0 keeps the bitmap's own colours), with the mask as
/// opacity, written over `layer`, a texture to take the result.
pub(super) fn compose(
    texture: &Texture,
    mask: &Texture,
    remap: usize,
    colors: [[u8; 3]; 3],
    mut layer: Texture,
) -> Result<Texture> {
    ensure!(texture.width == mask.width && texture.height == mask.height, "vinyl/mask dimensions differ");
    let mut rgba = decode_rgba8(texture).context("vinyl pixels cannot be decoded")?;
    let mask = decode_rgba8(mask).context("vinyl mask cannot be decoded")?;
    ensure!(remap <= 3, "vinyl uses unsupported color weights");
    for (pixel, mask) in rgba.as_chunks_mut::<4>().0.iter_mut().zip(mask.as_chunks::<4>().0) {
        if remap > 0 {
            let weights = [pixel[0], pixel[1], pixel[2]];
            for (channel, value) in pixel[..3].iter_mut().enumerate() {
                *value = (0..remap)
                    .map(|i| u32::from(weights[i]) * u32::from(colors[i][channel]))
                    .sum::<u32>()
                    .div_ceil(255)
                    .min(255) as u8;
            }
        }
        pixel[3] = mask[0];
    }
    layer.width = texture.width;
    layer.height = texture.height;
    layer.data = rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
    layer.alpha_usage = AlphaUsage::None;
    layer.format = PixelFormat::Argb8888;
    Ok(layer)
}

pub(super) fn repaint(model: &mut CarModel) -> Result<()> {
    let Some(layer) = model.textures.get(&bstring_hash(LAYER)) else {
        return Ok(());
    };
    let rgb = model.paint.as_ref().context("vinyl requires body paint")?.rgb;
    let mut skin = layer.clone();
    for p in skin.data.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        for (i, c) in rgb.into_iter().rev().enumerate() {
            p[i] = ((u32::from(p[i]) * a + u32::from(c) * (255 - a) + 127) / 255) as u8;
        }
        p[3] = 255;
    }
    let names: Vec<_> = model
        .textures
        .values()
        .filter(|t| t.name.ends_with("_SKIN1") && t.name != "SPOILER_SKIN1")
        .map(|t| t.name.clone())
        .collect();
    for name in names {
        let mut skin = skin.clone();
        skin.name_hash = bstring_hash(&name);
        skin.name = name;
        model.textures.insert(skin.name_hash, skin);
    }
    Ok(())
}
