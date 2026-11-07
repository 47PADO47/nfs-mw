//! A track pack into the files of a second world: the cells of the ground as section files (solids, textures, collision), the
//! manifests that place them, and the world's own manifest. The files go under `<out>/files/WORLDS/<ID>/` and
//! `<out>/worlds/` (`docs/specs/second-world.md`).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, bail};
use blackbox_modding::content::WORLDS_FOLDER;
use blackbox_tpk::TextureSource;

use crate::cells::{self, Cell};
use crate::collision::soup;
use crate::manifest::{self, MAX_CELLS, Place};
use crate::model::solid_of;
use crate::pack::{self, Pack};
use crate::textures::{Pictures, flat_colour};

/// The side of a cell, metres: sections of this size hold a few thousand ground triangles and 5 to 15 pictures.
pub const CELL_METRES: f32 = 384.0;

/// What an import did.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The world's id.
    pub world: String,
    pub cells: usize,
    pub triangles: usize,
    pub solids: usize,
    pub pictures: usize,
    /// Triangles that are ground a wheel can touch, and the ones too wide to be collision.
    pub collision_triangles: usize,
    pub collision_skipped: usize,
    /// The size of the section files written, bytes.
    pub bytes: u64,
    /// Pictures that could not be converted, with the reason; their materials are drawn flat.
    pub missing_pictures: Vec<(String, String)>,
}

/// Imports the pack in `pack_dir` into `out`, stopping early (with an error) when `cancel` is set.
pub fn import(pack_dir: &Path, out: &Path, max_texture: u32, cancel: &AtomicBool) -> Result<Report> {
    let pack = pack::read(pack_dir).with_context(|| format!("reading the pack {}", pack_dir.display()))?;
    let id = manifest::world_id(&pack.track);
    if id.is_empty() {
        bail!("the track name {:?} gives no world id", pack.track);
    }
    let cells = cells::bin(&pack, CELL_METRES);
    if cells.len() > MAX_CELLS {
        bail!("{} cells are more than the section numbers hold ({MAX_CELLS})", cells.len());
    }
    let (pictures, missing_pictures) = Pictures::load(&pack_dir.join("tex"), &stems_of(&pack), max_texture);
    let files = out.join("files").join("WORLDS").join(id.to_ascii_uppercase());
    let worlds = out.join(WORLDS_FOLDER).join(&id);
    for folder in [&files, &worlds] {
        std::fs::create_dir_all(folder).with_context(|| format!("creating {}", folder.display()))?;
    }
    let mut report = Report {
        world: id.clone(),
        cells: cells.len(),
        triangles: pack.triangles(),
        solids: 0,
        pictures: pictures_count(&pack, &pictures),
        collision_triangles: 0,
        collision_skipped: 0,
        bytes: 0,
        missing_pictures,
    };
    for (index, cell) in cells.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            bail!("the import was cancelled");
        }
        write_cell(&pack, &pictures, &id, index, cell, out, &mut report)?;
    }
    write_world(&pack, &id, out)?;
    Ok(report)
}

/// The lower-case stems of the pictures the materials name, each once.
fn stems_of(pack: &Pack) -> Vec<String> {
    let stems: BTreeSet<String> = pack.materials.iter().filter_map(|m| m.texture.clone()).collect();
    stems.into_iter().collect()
}

fn pictures_count(pack: &Pack, pictures: &Pictures) -> usize {
    stems_of(pack).iter().filter(|stem| pictures.get(stem).is_some()).count()
}

/// The texture a material is drawn with: its picture, or a flat colour when it has none or the picture is missing.
fn texture_for(pack: &Pack, pictures: &Pictures, material: usize) -> TextureSource {
    let m = &pack.materials[material];
    match m.texture.as_deref().and_then(|stem| pictures.get(stem)) {
        Some(texture) => texture.clone(),
        None => flat_colour(m.color),
    }
}

fn write_cell(
    pack: &Pack,
    pictures: &Pictures,
    id: &str,
    index: usize,
    cell: &Cell,
    out: &Path,
    report: &mut Report,
) -> Result<()> {
    let key = manifest::cell_key(index);
    let number = manifest::section_number(index);
    let mut sources = Vec::new();
    let mut textures: Vec<TextureSource> = Vec::new();
    let mut placed = Vec::new();
    for (at, part) in cell.parts.iter().enumerate() {
        let texture = texture_for(pack, pictures, part.material);
        let solid = solid_of(&format!("{}_{at}", key.to_ascii_uppercase()), part, &texture.name);
        if !textures.iter().any(|known| known.name == texture.name) {
            textures.push(texture);
        }
        placed.push((solid.source.name.clone(), solid.centre));
        sources.push(solid.source);
    }
    let ground = soup(&cell.ground, &pack.materials);
    let section = crate::section::build(&key.to_ascii_uppercase(), number, &sources, &textures, &ground)?;
    let file = manifest::cell_file(id, index);
    let path = out.join("files").join(&file);
    std::fs::write(&path, &section.bytes).with_context(|| format!("writing {}", path.display()))?;
    let places: Vec<Place<'_>> = placed
        .iter()
        .enumerate()
        .map(|(at, (model, centre))| Place { model, pos: *centre, collision: at == 0 && !ground.is_empty() })
        .collect();
    let text = manifest::cell_manifest(&file, number, cell.anchor(), &places);
    let manifest_path = out.join(WORLDS_FOLDER).join(id).join(format!("{key}.toml"));
    std::fs::write(&manifest_path, text).with_context(|| format!("writing {}", manifest_path.display()))?;
    report.solids += sources.len();
    report.collision_triangles += section.collision.triangles;
    report.collision_skipped += section.collision.skipped;
    report.bytes += section.bytes.len() as u64;
    Ok(())
}

/// The world's manifest: the name, and the start at the first slot of the grid (the middle of the lap's first sample when the
/// pack has no grid).
fn write_world(pack: &Pack, id: &str, out: &Path) -> Result<()> {
    let start = pack.lap.grid.first().copied().unwrap_or([0.0, 0.0, 30.0, 0.0]);
    let name = format!("{} (RIDE 6)", pack.track);
    let text = manifest::world_manifest(&name, [start[0], start[1], start[2]], start[3]);
    let path = out.join(WORLDS_FOLDER).join(format!("{id}.toml"));
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
