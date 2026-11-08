//! What the rider wears: the gear a mod lists in `RIDER/GEAR/gear.json` put on his skeleton in place of his own
//! (`docs/specs/ride6-rider-gear.md`, `docs/formats/ride6-rider-gear.md`).
//!
//! A piece of gear is a mesh pack of its own (one skinned mesh on the mesh's own bones, which are found by name in the rider's
//! skeleton) and a picture, or for gloves a colour. [`dress`] adds the meshes of the choice to the rider's pack, takes the
//! entries of `parts.json` of the gear they replace out and puts the gear's in, so the planning of the parts
//! (`parts::plan_without`) is the same for stock and for worn gear: an entry names a mesh and one of its sections' roles.
//! A picture is named for the loader: `gear:<path>` is a block-compressed picture of the gear folder ([`upload`]),
//! `colour:rrggbb` a flat colour.

use anyhow::{Context, Result, bail, ensure};
use blackbox_gfx::{PixelFormat, RenderBackend, TextureDesc, TextureHandle};
use blackbox_skeleton::{Pack, Skeleton};
use game_install::GameDir;

use super::parts::{self, Entry, Picture};
use crate::garage::rider_gear::catalogue::{Catalogue, FOLDER};
use crate::garage::rider_gear::{Choice, Part};

/// A section drawn with the item's picture, one drawn dark (the goggle band of an open helmet), the hands of a body.
const MAIN: &str = "main";
const GOGGLES: &str = "goggles";
const HANDS: &str = "hands";
/// The start of a picture name that is a file of the gear folder, and of one that is a flat colour.
const PICTURE: &str = "gear:";
const COLOUR: &str = "colour:";
/// The colour of a goggle band, which has no picture of its own. [ours]
const GOGGLE_GREY: [u8; 3] = [60, 60, 60];

/// The rider's parts for a choice.
pub struct Dressing {
    /// The entries of `parts.json` with those of the replaced gear taken out and the worn gear's put in.
    pub entries: Vec<Entry>,
    /// The meshes of the pack that are not drawn: the rider's own gear that other gear replaced.
    pub hidden: Vec<usize>,
}

fn colour(rgb: [u8; 3]) -> String {
    format!("{COLOUR}{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn entry(mesh: &str, slot: &str, picture: String, translucent: bool) -> Entry {
    Entry { mesh: mesh.to_owned(), slot: slot.to_owned(), picture, translucent, drawn: true }
}

/// The item of `list` whose id is `id`; none for the empty id (the rider's own gear).
fn item<'a, T>(list: &'a [T], id: &str, id_of: impl Fn(&T) -> &str) -> Option<&'a T> {
    match id.is_empty() {
        true => None,
        false => list.iter().find(|candidate| id_of(candidate) == id),
    }
}

/// The skinned mesh bones of `from` as bones of `to`, by name.
fn remap(map: &[Option<usize>], from: &Skeleton, to: &Skeleton) -> Vec<Option<usize>> {
    map.iter().map(|bone| bone.and_then(|i| from.bones().get(i)).and_then(|bone| to.find(&bone.name))).collect()
}

/// Adds the mesh pack at `path` of the gear folder to `pack`; the name its mesh has there.
fn load_mesh(dir: &GameDir, pack: &mut Pack, path: &str) -> Result<String> {
    let bytes = dir.read(&format!("{FOLDER}/{path}")).with_context(|| format!("reading {path}"))?;
    let mut gear = Pack::parse(&bytes).with_context(|| path.to_owned())?;
    let mut mesh = gear.meshes.pop().context("the pack has no mesh")?;
    mesh.bone_map = remap(&mesh.bone_map, &gear.skeleton, &pack.skeleton);
    let name = mesh.name.clone();
    pack.meshes.push(mesh);
    Ok(name)
}

/// [`load_mesh`], the failure logged: the item is then not worn.
fn push(dir: &GameDir, pack: &mut Pack, path: &str) -> Option<String> {
    match load_mesh(dir, pack, path) {
        Ok(name) => Some(name),
        Err(error) => {
            log::warn!("rider gear: {path} is not worn: {error:#}");
            None
        }
    }
}

/// Takes the rider's own meshes called `names` (of the first `base` meshes of the pack) out of the picture, with their entries.
fn replace(dressing: &mut Dressing, pack: &Pack, base: usize, names: &[String]) {
    for (index, mesh) in pack.meshes.iter().enumerate().take(base) {
        if names.contains(&mesh.name) {
            dressing.hidden.push(index);
        }
    }
    dressing.entries.retain(|entry| !names.contains(&entry.mesh));
}

/// The picture the rider's own suit wears: the entry of his first replaced body mesh.
fn stock_suit(stock: &[Entry], names: &[String]) -> Option<String> {
    let first = names.first()?;
    stock.iter().find(|entry| &entry.mesh == first && entry.drawn).map(|entry| entry.picture.clone())
}

struct Wearing<'a> {
    dir: &'a GameDir,
    catalogue: &'a Catalogue,
    choice: &'a Choice,
    stock: &'a [Entry],
    /// The meshes the rider's own pack had before any gear was added.
    base: usize,
}

impl Wearing<'_> {
    /// The body: the chosen one, or the catalogue's first when only the suit or the gloves are chosen (the stock suit mesh has no
    /// hands of its own to colour); its hands wear the gloves' colour, or the suit's picture when no gloves are chosen.
    fn body(&self, pack: &mut Pack, dressing: &mut Dressing) {
        let bodies = &self.catalogue.bodies;
        let body = match self.choice.body.is_empty() {
            true => bodies.first(),
            false => bodies.iter().find(|body| body.id == self.choice.body),
        };
        let Some(body) = body else { return };
        let suit = match item(&self.catalogue.suits, &self.choice.suit, |suit| suit.id.as_str()) {
            Some(suit) => Some(format!("{PICTURE}{}", suit.color)),
            None => stock_suit(self.stock, &self.catalogue.stock.body),
        };
        let Some(suit) = suit else { return };
        let hands = match item(&self.catalogue.gloves, &self.choice.gloves, |gloves| gloves.id.as_str()) {
            Some(gloves) => colour(gloves.rgb),
            None => suit.clone(),
        };
        let Some(name) = push(self.dir, pack, &body.mesh) else { return };
        replace(dressing, pack, self.base, &self.catalogue.stock.body);
        dressing.entries.push(entry(&name, MAIN, suit, false));
        dressing.entries.push(entry(&name, HANDS, hands, false));
    }

    /// The helmet and its visor; a helmet without a visor of its own takes the rider's visor away too, which fits his helmet only.
    fn helmet(&self, pack: &mut Pack, dressing: &mut Dressing) {
        let Some(helmet) = item(&self.catalogue.helmets, &self.choice.helmet, |helmet| helmet.id.as_str()) else {
            return;
        };
        let Some(name) = push(self.dir, pack, &helmet.mesh) else { return };
        let visor = helmet.visor.as_ref().and_then(|visor| Some((push(self.dir, pack, &visor.mesh)?, visor)));
        replace(dressing, pack, self.base, &self.catalogue.stock.helmet);
        replace(dressing, pack, self.base, &self.catalogue.stock.visor);
        dressing.entries.push(entry(&name, MAIN, format!("{PICTURE}{}", helmet.color), false));
        dressing.entries.push(entry(&name, GOGGLES, colour(GOGGLE_GREY), false));
        if let Some((visor_name, visor)) = visor {
            dressing.entries.push(entry(&visor_name, MAIN, format!("{PICTURE}{}", visor.color), true));
        }
    }

    fn boots(&self, pack: &mut Pack, dressing: &mut Dressing) {
        let Some(boots) = item(&self.catalogue.boots, &self.choice.boots, |boots| boots.id.as_str()) else { return };
        let Some(name) = push(self.dir, pack, &boots.mesh) else { return };
        replace(dressing, pack, self.base, &self.catalogue.stock.boots);
        dressing.entries.push(entry(&name, MAIN, format!("{PICTURE}{}", boots.color), false));
    }
}

/// The rider's parts for `choice`: the meshes of the gear are added to `pack`, the rider's own gear that they replace is hidden.
/// A choice that names nothing the catalogue has leaves the rider as he is.
pub fn dress(dir: &GameDir, pack: &mut Pack, stock: &[Entry], catalogue: &Catalogue, choice: &Choice) -> Dressing {
    let mut dressing = Dressing { entries: stock.to_vec(), hidden: Vec::new() };
    let worn = Wearing { dir, catalogue, choice, stock, base: pack.meshes.len() };
    if !choice.body.is_empty() || !choice.suit.is_empty() || !choice.gloves.is_empty() {
        worn.body(pack, &mut dressing);
    }
    worn.helmet(pack, &mut dressing);
    worn.boots(pack, &mut dressing);
    dressing
}

/// Whether a picture name is one of the gear's.
pub fn is_gear_picture(name: &str) -> bool {
    name.starts_with(PICTURE) || name.starts_with(COLOUR)
}

/// The three bytes of `rrggbb`.
fn parse_colour(hex: &str) -> Result<[u8; 3]> {
    ensure!(hex.len() == 6, "a colour has six hexadecimal digits");
    let byte = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).context("a colour has hexadecimal digits");
    Ok([byte(0)?, byte(2)?, byte(4)?])
}

/// The bytes of one 4 by 4 block of a block format.
fn block_bytes(format: PixelFormat) -> usize {
    match format {
        PixelFormat::Bc1 => 8,
        _ => 16,
    }
}

/// A picture of a gear file (`R6TX`, `docs/formats/ride6-rider-gear.md`): blocks and levels, or a PNG.
enum Decoded<'a> {
    Blocks(PixelFormat, Vec<(u32, u32, &'a [u8])>),
    Png(&'a [u8]),
}

fn take<'a>(bytes: &'a [u8], at: &mut usize, n: usize) -> Result<&'a [u8]> {
    let end = at.checked_add(n).context("length overflow")?;
    let slice = bytes.get(*at..end).context("the picture is cut short")?;
    *at = end;
    Ok(slice)
}

fn read_u32(bytes: &[u8], at: &mut usize) -> Result<u32> {
    let b = take(bytes, at, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Reads an `R6TX` file, keeping the levels a block format can show (4 by 4 pixels and above).
fn parse_picture(bytes: &[u8]) -> Result<Decoded<'_>> {
    let mut at = 0;
    ensure!(take(bytes, &mut at, 4)? == b"R6TX", "not an R6TX file");
    ensure!(read_u32(bytes, &mut at)? == 1, "unknown R6TX version");
    let code = read_u32(bytes, &mut at)?;
    let _srgb = take(bytes, &mut at, 1)?;
    let count = read_u32(bytes, &mut at)? as usize;
    ensure!((1..=17).contains(&count), "{count} mip levels");
    let format = match code {
        1 => PixelFormat::Bc1,
        2 => PixelFormat::Bc2,
        3 => PixelFormat::Bc3,
        255 => {
            let (_width, _height) = (read_u32(bytes, &mut at)?, read_u32(bytes, &mut at)?);
            let length = read_u32(bytes, &mut at)? as usize;
            return Ok(Decoded::Png(take(bytes, &mut at, length)?));
        }
        other => bail!("unknown picture format {other}"),
    };
    let mut mips = Vec::new();
    for _ in 0..count {
        let (width, height) = (read_u32(bytes, &mut at)?, read_u32(bytes, &mut at)?);
        let length = read_u32(bytes, &mut at)? as usize;
        let data = take(bytes, &mut at, length)?;
        ensure!(
            length == width.div_ceil(4) as usize * height.div_ceil(4) as usize * block_bytes(format),
            "a {width}x{height} level holds {length} bytes"
        );
        if width >= 4 && height >= 4 {
            mips.push((width, height, data));
        }
    }
    ensure!(!mips.is_empty(), "the picture has no level of 4 pixels or more");
    Ok(Decoded::Blocks(format, mips))
}

/// Uploads the picture a gear name stands for (a file of the gear folder, or a flat colour).
pub fn upload(renderer: &mut dyn RenderBackend, dir: &GameDir, name: &str) -> Result<TextureHandle> {
    if let Some(hex) = name.strip_prefix(COLOUR) {
        let [r, g, b] = parse_colour(hex)?;
        return Ok(parts::upload_picture(renderer, name, &Picture { width: 1, height: 1, rgba: vec![r, g, b, 255] }));
    }
    let path = name.strip_prefix(PICTURE).context("not a gear picture")?;
    let bytes = dir.read(&format!("{FOLDER}/{path}")).with_context(|| format!("reading {path}"))?;
    match parse_picture(&bytes).with_context(|| path.to_owned())? {
        Decoded::Png(png) => Ok(parts::upload_picture(renderer, name, &parts::decode_png(png)?)),
        Decoded::Blocks(format, mips) => {
            let (width, height, _) = mips[0];
            let levels = mips.iter().map(|(_, _, data)| *data).collect();
            Ok(renderer.create_texture(&TextureDesc { label: name, width, height, format, mips: levels }))
        }
    }
}

/// The parts of the rider a choice names that the catalogue has, for the log.
pub fn describe(choice: &Choice) -> String {
    Part::ALL
        .into_iter()
        .filter(|part| !choice.get(*part).is_empty())
        .map(|part| format!("{} {}", part.key(), choice.get(part)))
        .collect::<Vec<_>>()
        .join(", ")
}
