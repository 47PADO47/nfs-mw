//! The packs of the optional parts: one folder under `files/PARTS/R6/` per catalogue asset, with its own `GEOMETRY.BIN` and
//! `TEXTURES.BIN`, shared by every bike that can fit the part (`docs/modding/ride6-bike-upgrades.md`).
//!
//! A part is modelled in its own frame (the export's axes turned to the car's, nothing moved): the mount of each bike is the
//! `matrix` of the part's binding in the bike's manifest ([`crate::options_manifest`]). One folder per part, not one pack of
//! all of them, so that the game reads only the parts a bike wears, and a part shared by twenty bikes is on disk once.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use anyhow::{Context, Result, ensure};
use blackbox_solid::{SolidSource, write_geometry_pack};
use blackbox_tpk::write_tpk;

use crate::bake::{Options, Surfaces};
use crate::build::finish;
use crate::exports::Exports;
use crate::front_end::Take;
use crate::options::Kind;
use crate::partfile::{Part, PartTable};
use crate::silencer;
use crate::solids::{Context as GroupContext, Lists, Placer, part_groups};
use crate::textures::TexStore;

/// The prefix of the textures of every part: a texture that two parts share has one name, so a bike that wears both has it once.
const TEXTURE_PREFIX: &str = "R6";

/// The folder of the packs, under the mod's `files/`.
pub const PACK_ROOT: &str = "PARTS/R6";

/// The folder (under `files/`) of the pack of the asset `stem`.
pub fn pack_path(stem: &str) -> String {
    format!("{PACK_ROOT}/{}", stem.to_ascii_uppercase())
}

/// The name, without its level-of-detail letter, of the solid of the part `id` in its pack.
pub fn solid_name(id: &str) -> String {
    id.to_ascii_uppercase()
}

/// The part ids of the asset `stem` in the shared part table: the asset's own id, or `<stem>_front_<kind>` and `<stem>_back_<kind>`.
pub fn part_ids(table: &PartTable, stem: &str) -> Vec<String> {
    let (front, back) = (format!("{stem}_front_"), format!("{stem}_back_"));
    let owned = |id: &str| id == stem || id.starts_with(&front) || id.starts_with(&back);
    table.parts.iter().filter(|p| owned(&p.id)).map(|p| p.id.clone()).collect()
}

/// The longest side of a body texture for each kind of part (`Options::max_texture`: the rims' are half of it and the discs' a
/// quarter). **[choice]**: the exports hold 2048 pixel pictures for parts a few centimetres across, which would be 2.6 GB of
/// textures for the 957 parts; these sizes keep the whole set to some tens of megabytes.
pub fn max_texture(kind: Kind) -> u32 {
    match kind {
        Kind::Header => 128,
        Kind::Muffler | Kind::BarPad => 256,
        Kind::Rim | Kind::Brake => 1024,
    }
}

/// The solids a part of `kind` is written at: `(letter, level of detail of the export)`. **[choice]** The headers are 305 MB of
/// geometry at the four levels, the silencers 124 MB (550 and 330 parts), so a header has two levels (the exports' 2 and 3), a
/// silencer two (1 and 3) and a rim three; the game draws the nearest level a part has, so a letter left out costs nothing.
pub fn part_levels(kind: Kind) -> &'static [(char, u32)] {
    match kind {
        Kind::Header => &[('A', 2), ('B', 3)],
        Kind::Muffler => &[('A', 1), ('B', 3)],
        Kind::Rim => &[('A', 0), ('B', 2), ('D', 3)],
        Kind::Brake | Kind::BarPad => &[('A', 0), ('B', 1), ('C', 2), ('D', 3)],
    }
}

/// The export frame (glTF axes) as the car's: the one turn the stored solids have, without a move.
fn to_car([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, -z, y]
}

/// The shared part table and exports of the folder the exporter wrote.
pub struct Library {
    table: PartTable,
    exports: Exports,
    tex: PathBuf,
}

impl Library {
    pub fn open(dir: &Path) -> Result<Self> {
        let table = PartTable::read(dir)?;
        let exports = Exports::scan(&dir.join("glb"))?;
        Ok(Self { table, exports, tex: dir.join("tex") })
    }

    /// The solid of `part` at one level (`letter`, `level` of the export); `group`: the silencer of a twin it holds (`silencer.rs`),
    /// `None` for the whole mesh. `None` when the level has no triangle of the group.
    fn solid(
        &self,
        surfaces: &mut Surfaces,
        part: &Part,
        (letter, level): (char, u32),
        group: Option<u8>,
    ) -> Result<Option<SolidSource>> {
        let (mut groups, mut points, mut lists) = (Vec::new(), Vec::new(), Lists::default());
        let glb = self.exports.load(&part.export, level)?;
        let joints = silencer::joint_groups(&glb);
        let take = group.map_or(Take::All, |number| Take::Group(number, &joints));
        let mut ctx = GroupContext { table: &self.table, surfaces, car: TEXTURE_PREFIX, lists: &mut lists };
        groups.extend(part_groups(&mut ctx, &glb, (&Placer { part, to_solid: &to_car }, take), &mut points)?);
        if groups.is_empty() {
            return Ok(None);
        }
        let name = silencer::solid_name(&solid_name(&part.id), group.unwrap_or(1));
        finish(&format!("{name}_{letter}"), groups, lists).map(Some)
    }

    /// The silencers of a part of `kind`: one for every part but a twin silencer, which is one solid for each of its cans.
    fn silencers(&self, part: &Part, kind: Kind) -> Result<Vec<Option<u8>>> {
        if kind != Kind::Muffler {
            return Ok(vec![None]);
        }
        let finest = self.exports.load(&part.export, part_levels(kind)[0].1)?;
        Ok(silencer::groups(&finest).into_iter().map(Some).collect())
    }

    /// Build the pack of the asset `stem` of `kind`: `(GEOMETRY.BIN, TEXTURES.BIN)`.
    pub fn build(&self, stem: &str, kind: Kind, base: Options) -> Result<(Vec<u8>, Vec<u8>)> {
        let ids = part_ids(&self.table, stem);
        ensure!(!ids.is_empty(), "{stem}: the shared part table has no part of it");
        let options = Options { max_texture: max_texture(kind).min(base.max_texture.max(4)), ..base };
        let mut surfaces = Surfaces::new(TexStore::open(&self.tex)?, options);
        let mut solids = Vec::new();
        for id in &ids {
            let part = self.table.parts.iter().find(|p| &p.id == id).context("a part vanished")?;
            for group in self.silencers(part, kind).with_context(|| format!("{id}: its silencers"))? {
                for &(letter, level) in part_levels(kind) {
                    let made = self
                        .solid(&mut surfaces, part, (letter, level), group)
                        .with_context(|| format!("{id} level {letter}"))?;
                    solids.extend(made);
                }
            }
        }
        let geometry = write_geometry_pack(&solids).with_context(|| format!("writing the geometry of {stem}"))?;
        let textures = write_tpk(&stem.to_ascii_uppercase(), &surfaces.palette.into_textures())
            .with_context(|| format!("writing the textures of {stem}"))?;
        Ok((geometry, textures))
    }
}

/// What writing the packs did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Written {
    pub packs: usize,
    pub bytes: u64,
    /// `(asset stem, reason)` of each pack that could not be built; the bikes that list it do not offer it.
    pub failed: Vec<(String, String)>,
}

/// Write the packs of `stems` into `out/files/PARTS/R6/`, a few at a time. A part that fails is listed, not fatal.
pub fn write_all(
    library: &Library,
    out: &Path,
    stems: &[(String, Kind)],
    (base, threads): (Options, usize),
    cancel: &AtomicBool,
) -> Result<Written> {
    let next = AtomicUsize::new(0);
    let written = Mutex::new(Written::default());
    thread::scope(|scope| -> Result<()> {
        for _ in 0..threads.max(1) {
            let worker = thread::Builder::new().stack_size(crate::batch::STACK_BYTES).spawn_scoped(scope, || {
                loop {
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some((stem, kind)) = stems.get(at) else { return };
                    if cancel.load(Ordering::Relaxed) {
                        return;
                    }
                    let result = write_one(library, out, stem, *kind, base);
                    let mut written = written.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    match result {
                        Ok(bytes) => {
                            written.packs += 1;
                            written.bytes += bytes;
                        }
                        Err(error) => written.failed.push((stem.clone(), format!("{error:#}"))),
                    }
                }
            });
            worker.context("starting a part thread")?;
        }
        Ok(())
    })?;
    ensure!(!cancel.load(Ordering::Relaxed), "the import was stopped");
    let mut written = written.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);
    written.failed.sort();
    Ok(written)
}

fn write_one(library: &Library, out: &Path, stem: &str, kind: Kind, base: Options) -> Result<u64> {
    let (geometry, textures) = library.build(stem, kind, base)?;
    let dir = out.join("files").join(pack_path(stem));
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    std::fs::write(dir.join("GEOMETRY.BIN"), &geometry).context("writing GEOMETRY.BIN")?;
    std::fs::write(dir.join("TEXTURES.BIN"), &textures).context("writing TEXTURES.BIN")?;
    Ok((geometry.len() + textures.len()) as u64)
}

#[cfg(test)]
#[path = "options_pack_tests.rs"]
mod tests;
