//! The tail pipe effects of the 2012 game: which effect plays when the nitrous lights, burns and when the exhaust pops, and
//! the emitters and pictures each one is made of (evidence: `docs/formats/chameleon-nitrous.md`, "Tail pipe effects").
//!
//! A car's `VehicleVfxBehaviour` object lists `SpotEffect` records: a locator group (the tail pipes), a bit of the light-state
//! word and a reference to an effect (`BearEffect`, kind `0x301`, in `GLOBALEFFECTS.BNDL`). The state word is built by the
//! game each frame; the bits that name the nitrous are the ones the executable sets from the vehicle's nitrous state byte:
//! bit 9 when it is 1 (the start), bit 20 when it is 3 (the burn), and bit 14 when the throttle is off and the pop timer runs
//! (the blow-off). An effect is a list of emitters, each with the attributes it keeps per particle (a `SubUV` one is a
//! flipbook) and one material, whose first texture is the picture.
//!
//! The emitters' programs are read by `chameleon-effects` (spawn rate, lifetime, radius curve, motion, colour and alpha over
//! the life); this module finds the effects of the cars and the pictures of their emitters.

use anyhow::{Result, bail};
use blackbox_tpk::{AlphaUsage, PixelFormat as TpkFormat, Texture as TpkTexture, decode_mip_rgba8};
use chameleon_bundle::{Bundle, ResourceKind};
use chameleon_gfx::{PixelFormat, Texture};

/// The file with the effects.
pub const GLOBAL_EFFECTS: &str = "GLOBALEFFECTS.BNDL";

/// The bits of the light-state word of the nitrous effects.
pub const START_BIT: u32 = 9;
pub const BURN_BIT: u32 = 20;
pub const POP_BIT: u32 = 14;

const SPOT_TYPE: &str = "Genesys.Gen.VehicleVfxBehaviour.SpotEffect";
/// Where a `SpotEffect` keeps its effect reference, its locator group and its bit.
const SPOT_EFFECT: u32 = 0x0C;
const SPOT_GROUP: usize = 0x14;
const SPOT_BIT: usize = 0x18;
const TYPE_FLAG: u32 = 0x8000_0000;

/// One `SpotEffect` of a car.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spot {
    pub effect: u64,
    pub group: u32,
    pub bit: u32,
}

/// The effects of the three moments, ids of `GLOBALEFFECTS.BNDL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roles {
    pub start: u64,
    pub burn: u64,
    pub pop: u64,
}

/// The `SpotEffect` records of a car bundle.
pub fn spots(bundle: &Bundle) -> Vec<Spot> {
    let types = type_ids(bundle);
    let Some(spot_type) = types else { return Vec::new() };
    let mut out = Vec::new();
    let object = ResourceKind::GenesysObject.id();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != object || entry.import_count == 0 {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        let main = &resource.blocks[0];
        for import in resource.imports.iter().filter(|i| i.offset & TYPE_FLAG != 0 && spot_type.contains(&i.id)) {
            let at = (import.offset & !TYPE_FLAG) as usize;
            let slot = (import.offset & !TYPE_FLAG) + SPOT_EFFECT;
            let Some(effect) = resource.imports.iter().find(|r| r.offset & !TYPE_FLAG == slot) else { continue };
            let (Some(group), Some(bit)) = (word(main, at + SPOT_GROUP), word(main, at + SPOT_BIT)) else { continue };
            out.push(Spot { effect: effect.id, group, bit });
        }
    }
    out
}

fn word(block: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(block.get(at..at + 4)?.try_into().ok()?))
}

/// The ids of the `SpotEffect` type resources of the bundle, or `None` when it has none.
fn type_ids(bundle: &Bundle) -> Option<Vec<u64>> {
    let kind = ResourceKind::GenesysType.id();
    let mut ids = Vec::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        if type_name(&resource.blocks[0]).as_deref() == Some(SPOT_TYPE) {
            ids.push(entry.id);
        }
    }
    (!ids.is_empty()).then_some(ids)
}

/// The name of a Genesys type: a string whose offset is at 0x20 of the main block, 8 bytes in.
fn type_name(main: &[u8]) -> Option<String> {
    let start = word(main, 0x20)? as usize + 8;
    let tail = main.get(start..)?;
    let end = tail.iter().position(|&byte| byte == 0)?;
    String::from_utf8(tail[..end].to_vec()).ok()
}

/// The effects of the start, the burn and the pop: those of the group that has a pop effect.
pub fn roles(spots: &[Spot]) -> Option<Roles> {
    let group = spots.iter().find(|spot| spot.bit == POP_BIT)?.group;
    // Some cars list two effects for the burn in the pipes' group (a cloud first, then the glow the Porsche has alone): the
    // last one listed is the glow in all of them.
    let of = |bit: u32| spots.iter().rev().find(|spot| spot.group == group && spot.bit == bit).map(|spot| spot.effect);
    Some(Roles { start: of(START_BIT)?, burn: of(BURN_BIT)?, pop: of(POP_BIT)? })
}

/// One emitter of an effect that draws particles: its picture, and what the effect's bytes say of it (`chameleon-effects`).
#[derive(Debug, Clone, PartialEq)]
pub struct Emitter {
    pub picture: Picture,
    /// The emitter plays a flipbook.
    pub flipbook: bool,
    pub fx: chameleon_effects::Emitter,
}

/// A decoded picture, RGBA8.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The operations that draw an emitter's particles as sprites.
const SPRITES: [u8; 3] = [
    chameleon_effects::ops::RENDER_SPRITE,
    chameleon_effects::ops::RENDER_SPRITE_SUB_UV,
    chameleon_effects::ops::RENDER_SPRITE_UV_DISTORTION,
];

/// The emitters of the effect `id` that draw sprites, in order, each with its picture. The sprite emitters are matched with the
/// effect's material imports in order; an emitter with no sprite (the light) has no material.
pub fn emitters(bundle: &Bundle, id: u64) -> Result<Vec<Emitter>> {
    let Some(index) = bundle.find(id) else { bail!("the effect {id:#x} is not in the bundle") };
    let resource = bundle.resource(index)?;
    let effect = chameleon_effects::Effect::parse(&resource.blocks[0]).map_err(|error| anyhow::anyhow!("{error}"))?;
    let drawing: Vec<chameleon_effects::Emitter> =
        effect.emitters.into_iter().filter(|e| e.render_op().is_some_and(|op| SPRITES.contains(&op))).collect();
    let mut materials: Vec<_> = resource
        .imports
        .iter()
        .filter(|import| {
            bundle.find(import.id).is_some_and(|at| bundle.entries()[at].kind == ResourceKind::Material.id())
        })
        .collect();
    materials.sort_by_key(|import| import.offset);
    if materials.len() != drawing.len() {
        bail!("the effect {id:#x} has {} sprite emitters and {} materials", drawing.len(), materials.len());
    }
    let mut out = Vec::new();
    for (fx, import) in drawing.into_iter().zip(materials) {
        let picture = material_picture(bundle, import.id)?;
        out.push(Emitter { picture, flipbook: fx.flipbook(), fx });
    }
    Ok(out)
}

/// The picture of the first texture of a material.
fn material_picture(bundle: &Bundle, material: u64) -> Result<Picture> {
    let resource =
        bundle.resource(bundle.find(material).ok_or_else(|| anyhow::anyhow!("no material {material:#x}"))?)?;
    let main = &resource.blocks[0];
    let (Some(count), Some(table)) = (word(main, 0x20), word(main, 0x2C)) else { bail!("a short material") };
    if count == 0 {
        bail!("the material {material:#x} has no texture");
    }
    let Some(import) = resource.imports.iter().find(|i| i.offset & !TYPE_FLAG == table) else {
        bail!("the material {material:#x} names no first texture")
    };
    let texture =
        bundle.resource(bundle.find(import.id).ok_or_else(|| anyhow::anyhow!("no texture {:#x}", import.id))?)?;
    let texture = Texture::parse(&texture.blocks[0], &texture.blocks[1])?;
    decode(&texture)
}

/// A texture as RGBA8: the block formats through the shared decoder, plain RGBA as it is.
pub fn decode(texture: &Texture) -> Result<Picture> {
    let Some(mip) = texture.mip(0) else { bail!("the texture has no pixels") };
    let format = match texture.format {
        PixelFormat::Rgba8 => {
            return Ok(Picture { width: texture.width, height: texture.height, rgba: mip.to_vec() });
        }
        PixelFormat::Bgra8 => {
            let rgba = mip.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
            return Ok(Picture { width: texture.width, height: texture.height, rgba });
        }
        PixelFormat::Bc1 => TpkFormat::Dxt1,
        PixelFormat::Bc2 => TpkFormat::Dxt3,
        PixelFormat::Bc3 => TpkFormat::Dxt5,
        other => bail!("the texture format {other:?} is not read"),
    };
    let source = TpkTexture {
        name: String::new(),
        name_hash: 0,
        width: texture.width,
        height: texture.height,
        mip_levels: 1,
        format,
        compression_type: 0,
        alpha_usage: AlphaUsage::None,
        alpha_sorting: false,
        alpha_blend: 0,
        flags: 0,
        data: mip.to_vec(),
        palette: Vec::new(),
    };
    let Some(rgba) = decode_mip_rgba8(&source, 0) else { bail!("the texture does not decode") };
    Ok(Picture { width: texture.width, height: texture.height, rgba })
}

/// A picture as a PNG file.
pub fn png_bytes(picture: &Picture) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, picture.width, picture.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&picture.rgba)?;
    writer.finish()?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
