//! The `CARS/` folders of the install, and the solids of the shared part folders (`WHEELS`, `BRAKES`, `ROOF`, ...),
//! each read once per session.
//!
//! A car load lists the folders to find its shared parts, and reads their `GEOMETRY.BIN` when its own geometry lacks
//! a part. [`GameDir::subdirectories`] walks the whole file index, and parsing the geometry is the costly part, so
//! both are kept for the session.
//!
//! The listing is memoised on `GLOBAL/GLOBALB.LZC`, which every install has. A listing also depends on the overlays of
//! the [`GameDir`] it was read from, so an entry is used only by a directory whose layers (root and file count) match
//! those it was read with; any other directory lists afresh. The shared solids are keyed on the `GEOMETRY.BIN` files
//! they come from, so they follow the overlays on their own.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use blackbox_solid::Solid;
use game_install::GameDir;

use super::read_solids;
use crate::{memo, shared};

/// The `CARS/` folders of one directory, as the listing found them.
struct CarFolders {
    /// The layers the listing was read with: each layer's root and file count.
    layers: Vec<(PathBuf, usize)>,
    /// The folders that have a `GEOMETRY.BIN`: the car folders (see [`cars`]).
    cars: Vec<String>,
    /// The folders that are shared parts, whether or not they have a `GEOMETRY.BIN`.
    shared: Vec<String>,
}

impl CarFolders {
    fn scan(dir: &GameDir) -> Self {
        let subdirs = dir.subdirectories("CARS");
        let cars = subdirs.iter().filter(|car| dir.exists(&format!("CARS/{car}/GEOMETRY.BIN"))).cloned().collect();
        let shared_folders = subdirs.iter().filter(|folder| super::is_shared_part_folder(folder)).cloned().collect();
        Self { layers: layers_of(dir), cars, shared: shared_folders }
    }
}

fn layers_of(dir: &GameDir) -> Vec<(PathBuf, usize)> {
    dir.layers().iter().map(|layer| (layer.root.to_path_buf(), layer.file_count)).collect()
}

/// The listing of `dir`, read once per session when `dir` has the layers the listing was read with.
fn listing(dir: &GameDir) -> Arc<CarFolders> {
    if !dir.exists(shared::GLOBALB_FILE) {
        return Arc::new(CarFolders::scan(dir));
    }
    match memo(dir, &[shared::GLOBALB_FILE], || Ok(CarFolders::scan(dir))) {
        Ok(hit) if hit.layers == layers_of(dir) => hit,
        _ => Arc::new(CarFolders::scan(dir)),
    }
}

/// The car folders of `dir`: the `CARS/` folders that have a `GEOMETRY.BIN`, in listing order.
pub fn cars(dir: &GameDir) -> Vec<String> {
    listing(dir).cars.clone()
}

/// The solids of every shared part folder of `dir`, read once per session for the folders' current files.
pub fn shared_solids(dir: &GameDir) -> Result<Arc<SharedSolids>> {
    let folders = listing(dir);
    let paths: Vec<String> = folders.shared.iter().map(|folder| format!("CARS/{folder}/GEOMETRY.BIN")).collect();
    let files: Vec<&str> = paths.iter().map(String::as_str).collect();
    memo(dir, &files, || SharedSolids::read(dir, &folders.shared))
}

/// Every solid of the shared part folders, by name hash. Where two folders hold the same name, the first folder in
/// listing order gives the solid.
pub struct SharedSolids {
    by_hash: HashMap<u32, (String, Arc<Solid>)>,
}

impl SharedSolids {
    /// The solid with this name hash, and the folder it came from.
    pub fn get(&self, hash: u32) -> Option<(&str, &Arc<Solid>)> {
        self.by_hash.get(&hash).map(|(folder, solid)| (folder.as_str(), solid))
    }

    fn read(dir: &GameDir, folders: &[String]) -> Result<Self> {
        let mut by_hash = HashMap::new();
        for folder in folders {
            for solid in read_solids(dir, folder)? {
                by_hash.entry(solid.name_hash).or_insert_with(|| (folder.clone(), Arc::new(solid)));
            }
        }
        Ok(Self { by_hash })
    }
}
