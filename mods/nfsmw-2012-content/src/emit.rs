//! Writing the mod's generated content: one `cars/<id>.toml` per car and the car files under `files/CARS/<ID>/`, next to
//! the mod's `mod.toml`. The game reads both folders after the mod has loaded (`docs/modding/api-content.md`, "Cars").

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Role;
use crate::performance::Performance;

/// One imported car, as the mod folder records it.
#[derive(Debug, Clone, PartialEq)]
pub struct CarEntry {
    /// The car folder, upper case.
    pub id: String,
    /// The stock car it clones.
    pub clone: String,
    pub name: String,
    pub manufacturer: String,
    pub role: Role,
    /// How the car drives compared with its clone.
    pub performance: Performance,
    /// The car's own paints: the `[customization]` table text, empty when the car has none (`paints::manifest_table`).
    pub customization: String,
}

/// The files of one car.
pub struct CarFiles<'a> {
    pub geometry: &'a [u8],
    pub textures: &'a [u8],
    /// The car's own engine loops, if it has any.
    pub engine: Option<&'a EngineFiles>,
}

/// A car's engine loops and the engine speeds they stand for.
pub struct EngineFiles {
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    pub layers: Vec<crate::engine::Layer>,
}

/// The generated folders of a mod folder.
pub struct ModFolder {
    root: PathBuf,
}

impl ModFolder {
    /// Start a fresh set of generated content in `root`: the folders an earlier run wrote (`files/`, `cars/`) are
    /// cleared. Both belong to the generator; nothing else of the mod folder is touched.
    pub fn create(root: &Path) -> Result<Self> {
        for part in ["files", "cars"] {
            let path = root.join(part);
            if path.exists() {
                std::fs::remove_dir_all(&path).with_context(|| format!("clearing {}", path.display()))?;
            }
        }
        std::fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
        Ok(Self { root: root.to_owned() })
    }

    pub fn add_car(&self, car: &CarEntry, files: &CarFiles) -> Result<()> {
        let folder = self.root.join("files").join("CARS").join(&car.id);
        std::fs::create_dir_all(&folder)?;
        std::fs::write(folder.join("GEOMETRY.BIN"), files.geometry)?;
        std::fs::write(folder.join("TEXTURES.BIN"), files.textures)?;
        let engine = files.engine.map(|engine| self.add_engine(&car.id, engine)).transpose()?;
        let cars = self.root.join("cars");
        std::fs::create_dir_all(&cars)?;
        std::fs::write(
            cars.join(format!("{}.toml", car.id.to_ascii_lowercase())),
            car_manifest(car, engine.as_deref()),
        )?;
        Ok(())
    }

    /// Write the engine loops of the car `id` under `SOUND/MODENGINE/<ID>/` and return the manifest table that names them.
    fn add_engine(&self, id: &str, engine: &EngineFiles) -> Result<String> {
        let folder = self.root.join("files").join("SOUND").join("MODENGINE").join(id);
        std::fs::create_dir_all(&folder)?;
        let mut layers = Vec::new();
        for (at, layer) in engine.layers.iter().enumerate() {
            std::fs::write(folder.join(format!("L{at}.WAV")), &layer.wav)?;
            layers.push(format!("  {{ file = \"SOUND/MODENGINE/{id}/L{at}.WAV\", rpm = {:?} }},\n", layer.rpm));
        }
        Ok(format!(
            "\n[engine_sound]\nidle_rpm = {:?}\nredline_rpm = {:?}\nlayers = [\n{}]\n",
            engine.idle_rpm,
            engine.redline_rpm,
            layers.concat()
        ))
    }
}

fn quoted(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn car_manifest(car: &CarEntry, engine: Option<&str>) -> String {
    let mut text =
        format!("clone = {}\nrole = {}\nname = {}\n", quoted(&car.clone), quoted(car.role.as_str()), quoted(&car.name));
    if !car.manufacturer.is_empty() {
        text.push_str(&format!("manufacturer = {}\n", quoted(&car.manufacturer)));
    }
    text.push_str(&car.performance.manifest_table());
    text.push_str(&car.customization);
    text.push_str(engine.unwrap_or_default());
    text
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
            performance: Performance::default(),
            customization: "\n[customization]\npaints = [{ name = \"Red\", color = [200, 0, 0] }]\n".into(),
        }
    }

    #[test]
    fn the_car_manifest_is_valid_toml_the_game_accepts() {
        let text = car_manifest(&entry(), None);
        let parsed: toml::Table = toml::from_str(&text).unwrap();
        assert_eq!(parsed["clone"].as_str(), Some("BMWM3GTR"));
        assert_eq!(parsed["name"].as_str(), Some("Porsche 911 \"Carrera\" S 2012"));
        assert_eq!(parsed["role"].as_str(), Some("player"));
        assert_eq!(parsed["customization"]["paints"].as_array().map(Vec::len), Some(1));
    }

    #[test]
    fn a_new_run_clears_the_generated_folders_and_nothing_else() {
        let dir = std::env::temp_dir().join(format!("nfsmw-import2012-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.toml"), "keep").unwrap();
        let folder = ModFolder::create(&dir).unwrap();
        folder.add_car(&entry(), &CarFiles { geometry: b"g", textures: b"t", engine: None }).unwrap();
        assert!(dir.join("files/CARS/POR911CARR12/GEOMETRY.BIN").is_file());
        assert!(dir.join("cars/por911carr12.toml").is_file());
        ModFolder::create(&dir).unwrap();
        assert!(!dir.join("files").exists() && !dir.join("cars").exists());
        assert!(dir.join("mod.toml").is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
