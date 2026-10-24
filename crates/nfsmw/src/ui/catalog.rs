//! The FEng packages of the install, found by name.
//!
//! Screens live in several files (`FRONTEND/FrontB.lzc` has the menus, `GLOBAL/InGameB.bun` the in-game
//! screens). A [`Catalog`] reads the packages of the files it is given and finds one by its file name, in any
//! case. The first file that has a name wins.
//!
//! Each file is parsed once per session. Its packages stay in the session memo, so the menus, the HUD and the
//! EA Trax chyron share the work for the files they have in common.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use blackbox_chunk::find_all;
use blackbox_feng::Package;
use game_install::GameDir;
use nfsmw_data::{memo, shared_unwrapped};

const FENG_PACKAGE: u32 = 0x0003_0203;
const FENG_COMPRESSED: u32 = 0x0003_0210;

/// Files that hold screens, in the order they are searched.
pub const SCREEN_FILES: [&str; 7] = [
    "FRONTEND/FrontB.lzc",
    "GLOBAL/InGameB.bun",
    "GLOBAL/GlobalB.lzc",
    "GLOBAL/GLOBALA.BUN",
    "GLOBAL/INGAMEC.BUN",
    "GLOBAL/WIDESCREEN_GLOBAL.BUN",
    // The four split-screen HUD packages (`docs/specs/split-screen-hud.md`); no menu uses their names.
    "GLOBAL/InGameSplitScreen.bun",
];

/// Where a package came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub file: &'static str,
}

/// The packages of the files it was loaded from. The packages of each file are shared through the session memo,
/// so loading a catalog again costs only the lookups.
pub struct Catalog {
    /// The packages of each file that loaded, in search order.
    files: Vec<Arc<FilePackages>>,
    /// Lower-case file name -> (index into `files`, index into that file's packages) of the first package with it.
    index: HashMap<String, (usize, usize)>,
    entries: Vec<Entry>,
}

/// The packages of one file, in the order the file stores them. Its own memo type keeps it apart from any other
/// memo of a package list.
struct FilePackages {
    packages: Vec<Package>,
}

impl Catalog {
    /// Reads every package of `files`, each file once per session. A file that is missing or does not parse is skipped
    /// with a warning.
    pub fn load(dir: &GameDir, files: &[&'static str]) -> Self {
        let mut catalog = Self { files: Vec::new(), index: HashMap::new(), entries: Vec::new() };
        for &file in files {
            let file = nfsmw_data::shared::pack_file(dir, file);
            let packages = match memo(dir, &[file], || read_file(dir, file)) {
                Ok(packages) => packages,
                Err(e) => {
                    log::warn!("{file}: {e:#}");
                    continue;
                }
            };
            let source = catalog.files.len();
            for (at, package) in packages.packages.iter().enumerate() {
                let key = package.name.to_ascii_lowercase();
                if catalog.index.contains_key(&key) {
                    continue;
                }
                catalog.entries.push(Entry { name: package.name.clone(), file });
                catalog.index.insert(key, (source, at));
            }
            catalog.files.push(packages);
        }
        log::info!("UI: {} packages from {} files", catalog.index.len(), files.len());
        catalog
    }

    /// The package with this file name (`MainMenu.fng`), in any case.
    pub fn find(&self, name: &str) -> Option<&Package> {
        let &(source, at) = self.index.get(&name.to_ascii_lowercase())?;
        self.files[source].packages.get(at)
    }

    /// All packages, sorted by name.
    pub fn entries(&self) -> Vec<&Entry> {
        let mut all: Vec<&Entry> = self.entries.iter().collect();
        all.sort_by_key(|e| e.name.to_ascii_lowercase());
        all
    }
}

/// Parses the packages of one file, in stored order. A package that does not parse is skipped with a warning.
fn read_file(dir: &GameDir, file: &str) -> Result<FilePackages> {
    let data = shared_unwrapped(dir, file)?;
    let plain = find_all(&data, FENG_PACKAGE).into_iter().map(|c| Package::parse(c.payload));
    let packed =
        find_all(&data, FENG_COMPRESSED).into_iter().map(|c| Package::parse_compressed(c.payload).map(|(_, p)| p));
    let mut packages = Vec::new();
    for parsed in plain.chain(packed) {
        match parsed {
            Ok(package) => packages.push(package),
            Err(e) => {
                log::warn!("{file}: a package does not parse: {e}");
            }
        }
    }
    Ok(FilePackages { packages })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the install (`NFSMW_GAME_DIR`): the menus and the in-game screens are all found by name.
    #[test]
    fn a_real_install_has_the_menus() {
        let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        for name in [
            "MainMenu.fng",
            "mainmenu_sub.fng",
            "Options.fng",
            "Pause_Main.fng",
            "HUD_SingleRace.fng",
            "Track_Select.fng",
            "Track_Options.fng",
        ] {
            assert!(catalog.find(name).is_some(), "{name} is missing");
        }
        assert!(catalog.entries().len() >= 190);
    }
}
