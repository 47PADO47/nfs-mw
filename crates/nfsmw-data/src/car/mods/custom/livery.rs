//! A mod car's liveries: whole-body pictures in the layout of the car's paint surfaces, drawn over the body paint.
//!
//! The picture is a texture of the car's own `TEXTURES.BIN`, so it is authored for the car's own texture coordinates. It
//! is laid over the paint colour by its alpha: where it is opaque it replaces the paint, where it is transparent the
//! paint shows (the rule of the 2012 game's livery shader, docs/formats/chameleon-customization.md). The shop lists a
//! livery among the vinyls, because both are pictures over the paint; the result is the same flat-skin texture a vinyl
//! gives the paint surfaces, so [`crate::car::vinyl::repaint`] draws it.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use blackbox_hash::bstring_hash;
use blackbox_tpk::{AlphaUsage, PixelFormat, Texture, decode_rgba8_cutout};
use game_install::GameDir;

use super::{Customization, Kind, Livery, part_hash};
use crate::car::CarModel;
use crate::car::customization::{Catalog, Category, Choice};

/// The textures of `wanted` (by name hash) that the car's own `TEXTURES.BIN` holds. No such file: none.
pub(super) fn own_textures(dir: &GameDir, folder: &str, wanted: &HashSet<u32>) -> Result<HashMap<u32, Texture>> {
    let path = format!("CARS/{folder}/TEXTURES.BIN");
    if !dir.exists(&path) {
        return Ok(HashMap::new());
    }
    let packs =
        blackbox_tpk::read_texture_packs_filtered(&crate::read_unwrapped(dir, &path)?, |h| wanted.contains(&h))?;
    let mut found = HashMap::new();
    for texture in packs.into_iter().flat_map(|pack| pack.textures) {
        found.entry(texture.name_hash).or_insert(texture);
    }
    Ok(found)
}

/// Whether a picture can be drawn: a format the decoder reads, with the data of its first level.
fn usable(texture: &Texture) -> bool {
    let known =
        matches!(texture.format, PixelFormat::Dxt1 | PixelFormat::Dxt3 | PixelFormat::Dxt5 | PixelFormat::Argb8888);
    known && texture.mip(0).is_some()
}

/// Add the car's liveries to the vinyl choices of `catalog`. A livery whose picture the car's `TEXTURES.BIN` lacks, or
/// cannot be drawn, is not offered.
pub(super) fn offer(dir: &GameDir, folder: &str, custom: &Customization, catalog: &mut Catalog) -> Result<()> {
    if custom.liveries.is_empty() {
        return Ok(());
    }
    let wanted: HashSet<u32> = custom.liveries.iter().map(|livery| bstring_hash(&livery.texture)).collect();
    let found = own_textures(dir, folder, &wanted)?;
    for livery in &custom.liveries {
        if !found.get(&bstring_hash(&livery.texture)).is_some_and(usable) {
            log::warn!(
                "{folder}: the livery {:?} is not offered: the picture {:?} is missing from TEXTURES.BIN or cannot be read",
                livery.name,
                livery.texture
            );
            continue;
        }
        let hash = part_hash(folder, Kind::Livery, &livery.name);
        let label = livery.name.clone();
        catalog.choices[Category::Vinyl as usize].push(Choice { hash, label, language_hash: None, carbon: false });
        // A livery carries its own colours: no vinyl colour channel to recolour.
        catalog.vinyl_remaps.insert(hash, 0);
    }
    // A car that wears a livery from the start can take it off: the plain paint, shown with the other liveries.
    if custom.default_livery.is_some() {
        let hash = super::no_livery_hash(folder);
        let label = super::NO_LIVERY.to_owned();
        catalog.choices[Category::Vinyl as usize].push(Choice { hash, label, language_hash: None, carbon: false });
        catalog.vinyl_remaps.insert(hash, 0);
    }
    Ok(())
}

/// Put `livery` on the model: its picture over the body paint, on every paint surface. The body paint must be set first.
pub(super) fn apply(dir: &GameDir, folder: &str, livery: &Livery, model: &mut CarModel) -> Result<()> {
    let wanted = bstring_hash(&livery.texture);
    let found = own_textures(dir, folder, &HashSet::from([wanted]))?;
    let picture = found.get(&wanted).context("the livery picture is missing from TEXTURES.BIN")?;
    let paint = model.paint.as_ref().context("a livery needs the body paint")?;
    let layer = layer(picture, paint.texture(crate::car::vinyl::LAYER))?;
    model.textures.insert(layer.name_hash, layer);
    crate::car::vinyl::repaint(model)
}

/// The picture as the vinyl layer: its colours and its alpha, written over `template`, a texture to take the result.
pub(super) fn layer(picture: &Texture, mut template: Texture) -> Result<Texture> {
    let rgba = decode_rgba8_cutout(picture).context("the livery picture cannot be decoded")?;
    template.width = picture.width;
    template.height = picture.height;
    template.mip_levels = 1;
    template.data = rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
    template.alpha_usage = AlphaUsage::None;
    template.format = PixelFormat::Argb8888;
    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texture(width: u32, height: u32, format: PixelFormat, data: Vec<u8>) -> Texture {
        Texture {
            name: "PICTURE".to_owned(),
            name_hash: bstring_hash("PICTURE"),
            width,
            height,
            mip_levels: 1,
            format,
            compression_type: 0,
            alpha_usage: AlphaUsage::None,
            alpha_sorting: false,
            alpha_blend: 0,
            flags: 0,
            data,
            palette: Vec::new(),
        }
    }

    fn template() -> Texture {
        texture(4, 4, PixelFormat::Argb8888, vec![0; 64])
    }

    #[test]
    fn an_argb_picture_keeps_its_colours_and_its_alpha() {
        // B G R A as the game stores them: one opaque red pixel, one transparent green one.
        let picture = texture(2, 1, PixelFormat::Argb8888, vec![0, 0, 255, 255, 0, 255, 0, 0]);
        let layer = layer(&picture, template()).unwrap();
        assert_eq!((layer.width, layer.height, layer.mip_levels), (2, 1, 1));
        assert_eq!(layer.format, PixelFormat::Argb8888);
        assert_eq!(layer.data, vec![0, 0, 255, 255, 0, 255, 0, 0]);
    }

    #[test]
    fn a_dxt1_block_with_transparent_texels_gives_alpha_zero_and_an_opaque_one_gives_255() {
        // Colour0 below colour1 selects the three-colour mode, where index 3 is transparent; all sixteen texels use it.
        let transparent = [0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        // Colour0 above colour1 is the four-colour mode: every texel opaque (index 0).
        let opaque = [0xFF, 0xFF, 0x00, 0x00, 0, 0, 0, 0];
        let clear = layer(&texture(4, 4, PixelFormat::Dxt1, transparent.to_vec()), template()).unwrap();
        assert!(clear.data.chunks(4).all(|px| px[3] == 0));
        let solid = layer(&texture(4, 4, PixelFormat::Dxt1, opaque.to_vec()), template()).unwrap();
        assert!(solid.data.chunks(4).all(|px| px[3] == 255));
        assert_eq!(&solid.data[..3], &[255, 255, 255], "white: colour0 is 0xFFFF");
    }

    #[test]
    fn only_pictures_the_decoder_reads_are_offered() {
        assert!(usable(&texture(4, 4, PixelFormat::Dxt1, vec![0; 8])));
        assert!(!usable(&texture(4, 4, PixelFormat::Dxt1, vec![0; 4])), "short data");
        assert!(!usable(&texture(4, 4, PixelFormat::P8, vec![0; 16])), "a palette picture");
        assert!(!usable(&texture(4, 4, PixelFormat::Other(7), vec![0; 64])));
    }

    #[test]
    fn a_picture_that_cannot_be_decoded_is_an_error_not_a_blank() {
        let short = texture(4, 4, PixelFormat::Dxt5, vec![0; 3]);
        assert!(layer(&short, template()).is_err());
    }
}
