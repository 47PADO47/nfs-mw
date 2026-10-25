//! Writing the mod folder: `mod.toml`, one `cars/<id>.toml` per car and the car files under `files/CARS/<ID>/`.
//! See `docs/modding/api-content.md` ("Cars") for what the game reads.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::config::Role;

/// The mod's id and folder name.
pub const MOD_ID: &str = "mw2012.cars";
/// A file the importer leaves in the folders it owns, so it never clears a folder it did not make.
const MARKER: &str = ".nfsmw-import2012";

/// One imported car, as the mod folder records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarEntry {
    /// The car folder, upper case.
    pub id: String,
    /// The stock car it clones.
    pub clone: String,
    pub name: String,
    pub manufacturer: String,
    pub role: Role,
}

/// The files of one car.
pub struct CarFiles<'a> {
    pub geometry: &'a [u8],
    pub textures: &'a [u8],
}

/// The mod folder being written.
pub struct ModFolder {
    root: PathBuf,
}

impl ModFolder {
    /// Prepare `root`: create it, or clear the earlier import in it. A folder that exists, has other content and is
    /// not an earlier import is refused.
    pub fn create(root: &Path) -> Result<Self> {
        if root.exists() {
            let ours = root.join(MARKER).is_file();
            let empty = std::fs::read_dir(root)?.next().is_none();
            if !ours && !empty {
                bail!("{} exists and is not an earlier import; choose another output folder", root.display());
            }
            for part in ["files", "cars"] {
                let path = root.join(part);
                if ours && path.exists() {
                    std::fs::remove_dir_all(&path).with_context(|| format!("clearing {}", path.display()))?;
                }
            }
        }
        std::fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
        std::fs::write(root.join(MARKER), "written by nfsmw-import2012; the importer may clear files/ and cars/\n")?;
        Ok(Self { root: root.to_owned() })
    }

    pub fn add_car(&self, car: &CarEntry, files: &CarFiles) -> Result<()> {
        let folder = self.root.join("files").join("CARS").join(&car.id);
        std::fs::create_dir_all(&folder)?;
        std::fs::write(folder.join("GEOMETRY.BIN"), files.geometry)?;
        std::fs::write(folder.join("TEXTURES.BIN"), files.textures)?;
        let cars = self.root.join("cars");
        std::fs::create_dir_all(&cars)?;
        std::fs::write(cars.join(format!("{}.toml", car.id.to_ascii_lowercase())), car_manifest(car))?;
        Ok(())
    }

    pub fn finish(&self, count: usize) -> Result<()> {
        std::fs::write(self.root.join("mod.toml"), mod_manifest(count))?;
        Ok(())
    }
}

fn quoted(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn car_manifest(car: &CarEntry) -> String {
    let mut text =
        format!("clone = {}\nrole = {}\nname = {}\n", quoted(&car.clone), quoted(car.role.as_str()), quoted(&car.name));
    if !car.manufacturer.is_empty() {
        text.push_str(&format!("manufacturer = {}\n", quoted(&car.manufacturer)));
    }
    text
}

fn mod_manifest(count: usize) -> String {
    format!(
        "# Written by nfsmw-import2012 from the user's own installs of NFS Most Wanted (2012) and (2005).\n\
         # It holds {count} cars converted from game files: do not share this folder.\n\n\
         [mod]\nid = \"{MOD_ID}\"\nname = \"NFS Most Wanted (2012) cars\"\nversion = \"0.1.0\"\n\
         authors = [\"nfsmw-import2012\"]\n\n\
         [api]\nblackbox_modding = \"0.1\"\n\n\
         [games]\nsupported = [\"nfsmw\"]\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> CarEntry {
        CarEntry {
            id: "POR911CARR12".into(),
            clone: "BMWM3GTR".into(),
            name: "Porsche 911 \"Carrera\" S 2012".into(),
            manufacturer: "Porsche".into(),
            role: Role::Player,
        }
    }

    #[test]
    fn the_car_manifest_is_valid_toml_the_game_accepts() {
        let text = car_manifest(&entry());
        let parsed: toml::Table = toml::from_str(&text).unwrap();
        assert_eq!(parsed["clone"].as_str(), Some("BMWM3GTR"));
        assert_eq!(parsed["name"].as_str(), Some("Porsche 911 \"Carrera\" S 2012"));
        assert_eq!(parsed["role"].as_str(), Some("player"));
    }

    #[test]
    fn the_mod_manifest_parses() {
        let parsed: toml::Table = toml::from_str(&mod_manifest(3)).unwrap();
        assert_eq!(parsed["mod"]["id"].as_str(), Some(MOD_ID));
        assert_eq!(parsed["games"]["supported"][0].as_str(), Some("nfsmw"));
    }

    #[test]
    fn a_foreign_folder_is_never_cleared() {
        let dir = std::env::temp_dir().join(format!("nfsmw-import2012-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("precious.txt"), "keep").unwrap();
        assert!(ModFolder::create(&dir).is_err());
        assert!(dir.join("precious.txt").is_file());
        std::fs::remove_file(dir.join("precious.txt")).unwrap();
        let folder = ModFolder::create(&dir).unwrap();
        folder.add_car(&entry(), &CarFiles { geometry: b"g", textures: b"t" }).unwrap();
        folder.finish(1).unwrap();
        assert!(dir.join("files/CARS/POR911CARR12/GEOMETRY.BIN").is_file());
        assert!(ModFolder::create(&dir).is_ok());
        assert!(!dir.join("files").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
