//! The FEng packages of the install, found by name.
//!
//! Screens live in several files (`FRONTEND/FrontB.lzc` has the menus, `GLOBAL/InGameB.bun` the in-game
//! screens). A [`Catalog`] reads the packages of the files it is given and finds one by its file name, in any
//! case. The first file that has a name wins.

use std::collections::HashMap;

use blackbox_chunk::find_all;
use blackbox_feng::Package;
use game_install::GameDir;
use nfsmw_data::read_unwrapped;

const FENG_PACKAGE: u32 = 0x0003_0203;
const FENG_COMPRESSED: u32 = 0x0003_0210;

/// Files that hold screens, in the order they are searched.
pub const SCREEN_FILES: [&str; 6] = [
    "FRONTEND/FrontB.lzc",
    "GLOBAL/InGameB.bun",
    "GLOBAL/GlobalB.lzc",
    "GLOBAL/GLOBALA.BUN",
    "GLOBAL/INGAMEC.BUN",
    "GLOBAL/WIDESCREEN_GLOBAL.BUN",
];

/// Where a package came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub file: &'static str,
}

pub struct Catalog {
    packages: HashMap<String, Package>,
    entries: Vec<Entry>,
}

impl Catalog {
    /// Reads every package of `files`. A file that is missing or does not parse is skipped with a warning.
    pub fn load(dir: &GameDir, files: &[&'static str]) -> Self {
        let mut packages = HashMap::new();
        let mut entries = Vec::new();
        for &file in files {
            let data = match read_unwrapped(dir, file) {
                Ok(d) => d,
                Err(e) => {
                    log::warn!("{file}: {e:#}");
                    continue;
                }
            };
            let plain = find_all(&data, FENG_PACKAGE).into_iter().map(|c| Package::parse(c.payload));
            let packed = find_all(&data, FENG_COMPRESSED)
                .into_iter()
                .map(|c| Package::parse_compressed(c.payload).map(|(_, p)| p));
            for parsed in plain.chain(packed) {
                let package = match parsed {
                    Ok(p) => p,
                    Err(e) => {
                        log::warn!("{file}: a package does not parse: {e}");
                        continue;
                    }
                };
                let key = package.name.to_ascii_lowercase();
                if packages.contains_key(&key) {
                    continue;
                }
                entries.push(Entry { name: package.name.clone(), file });
                packages.insert(key, package);
            }
        }
        log::info!("UI: {} packages from {} files", packages.len(), files.len());
        Self { packages, entries }
    }

    /// The package with this file name (`MainMenu.fng`), in any case.
    pub fn find(&self, name: &str) -> Option<&Package> {
        self.packages.get(&name.to_ascii_lowercase())
    }

    /// All packages, sorted by name.
    pub fn entries(&self) -> Vec<&Entry> {
        let mut all: Vec<&Entry> = self.entries.iter().collect();
        all.sort_by_key(|e| e.name.to_ascii_lowercase());
        all
    }
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
        for name in ["MainMenu.fng", "mainmenu_sub.fng", "Options.fng", "Pause_Main.fng", "HUD_SingleRace.fng"] {
            assert!(catalog.find(name).is_some(), "{name} is missing");
        }
        assert!(catalog.entries().len() >= 190);
    }
}
