//! Writing the mod's generated content: one `cars/<id>.toml` per car and the car files under `files/CARS/<ID>/`, next to
//! the mod's `mod.toml`. The game reads both folders after the mod has loaded (`docs/modding/api-content.md`, "Cars").

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Role;
use crate::effects::EffectFiles;
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
}

/// The files of one car.
pub struct CarFiles<'a> {
    pub geometry: &'a [u8],
    pub textures: &'a [u8],
    /// The car's own engine loops, if it has any.
    pub engine: Option<&'a EngineFiles>,
    /// The car's own shifts, turbo, backfire and pops, if it has any.
    pub effects: Option<&'a EffectFiles>,
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
        let effects = files.effects.map(|effects| self.add_effects(&car.id, effects)).transpose()?;
        let sound: String = [engine, effects].into_iter().flatten().collect();
        let cars = self.root.join("cars");
        std::fs::create_dir_all(&cars)?;
        std::fs::write(
            cars.join(format!("{}.toml", car.id.to_ascii_lowercase())),
            car_manifest(car, (!sound.is_empty()).then_some(sound.as_str())),
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

impl ModFolder {
    /// Write the shifts, turbo, backfire and pops of the car `id` next to its engine loops and return the `[car_sound]`
    /// table that names them.
    fn add_effects(&self, id: &str, effects: &EffectFiles) -> Result<String> {
        let folder = self.root.join("files").join("SOUND").join("MODENGINE").join(id);
        std::fs::create_dir_all(&folder)?;
        let (table, files) = effects_table(id, effects);
        for (name, bytes) in files {
            std::fs::write(folder.join(name), bytes)?;
        }
        Ok(table)
    }
}

/// The `[car_sound]` table of the car `id` and the files it names (file name, bytes). Empty lists are left out, and a car
/// with no effect at all has no table.
fn effects_table<'a>(id: &str, effects: &'a EffectFiles) -> (String, Vec<(String, &'a [u8])>) {
    let lists = [
        ("shift", "SHIFT", &effects.shift),
        ("turbo_whine", "WHINE", &effects.turbo_whine),
        ("turbo_blowoff", "BLOWOFF", &effects.turbo_blowoff),
        ("backfire", "BACKFIRE", &effects.backfire),
        ("pops", "POP", &effects.pops),
    ];
    let mut table = String::new();
    let mut files = Vec::new();
    for (key, stem, wavs) in lists {
        if wavs.is_empty() {
            continue;
        }
        let mut paths = Vec::with_capacity(wavs.len());
        for (at, wav) in wavs.iter().enumerate() {
            let name = format!("{stem}{at}.WAV");
            paths.push(format!("\"SOUND/MODENGINE/{id}/{name}\""));
            files.push((name, wav.as_slice()));
        }
        table.push_str(&format!("{key} = [{}]\n", paths.join(", ")));
    }
    if table.is_empty() {
        return (table, files);
    }
    (format!("\n[car_sound]\n{table}"), files)
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
        }
    }

    #[test]
    fn the_car_manifest_is_valid_toml_the_game_accepts() {
        let text = car_manifest(&entry(), None);
        let parsed: toml::Table = toml::from_str(&text).unwrap();
        assert_eq!(parsed["clone"].as_str(), Some("BMWM3GTR"));
        assert_eq!(parsed["name"].as_str(), Some("Porsche 911 \"Carrera\" S 2012"));
        assert_eq!(parsed["role"].as_str(), Some("player"));
    }

    #[test]
    fn the_effects_table_names_every_file_it_writes_and_leaves_out_what_the_car_lacks() {
        let effects = EffectFiles {
            shift: vec![vec![1], vec![2]],
            backfire: vec![vec![3]],
            ..EffectFiles::default()
        };
        let (table, files) = effects_table("MYCAR", &effects);
        let parsed: toml::Table = toml::from_str(&table).unwrap();
        let sound = parsed["car_sound"].as_table().unwrap();
        assert_eq!(sound.keys().map(String::as_str).collect::<Vec<_>>(), ["backfire", "shift"]);
        let listed: Vec<&str> = sound["shift"].as_array().unwrap().iter().map(|path| path.as_str().unwrap()).collect();
        assert_eq!(listed, ["SOUND/MODENGINE/MYCAR/SHIFT0.WAV", "SOUND/MODENGINE/MYCAR/SHIFT1.WAV"]);
        let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["SHIFT0.WAV", "SHIFT1.WAV", "BACKFIRE0.WAV"]);
        assert_eq!(files[2].1, [3]);
        assert_eq!(effects_table("MYCAR", &EffectFiles::default()), (String::new(), Vec::new()));
    }

    #[test]
    fn the_manifest_with_sound_tables_is_valid_toml() {
        let effects = EffectFiles { pops: vec![vec![9]], ..EffectFiles::default() };
        let (table, _) = effects_table("POR911CARR12", &effects);
        let text = car_manifest(&entry(), Some(&table));
        let parsed: toml::Table = toml::from_str(&text).unwrap();
        assert_eq!(parsed["car_sound"]["pops"][0].as_str(), Some("SOUND/MODENGINE/POR911CARR12/POP0.WAV"));
    }

    #[test]
    fn a_new_run_clears_the_generated_folders_and_nothing_else() {
        let dir = std::env::temp_dir().join(format!("nfsmw-import2012-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.toml"), "keep").unwrap();
        let folder = ModFolder::create(&dir).unwrap();
        folder.add_car(&entry(), &CarFiles { geometry: b"g", textures: b"t", engine: None, effects: None }).unwrap();
        assert!(dir.join("files/CARS/POR911CARR12/GEOMETRY.BIN").is_file());
        assert!(dir.join("cars/por911carr12.toml").is_file());
        ModFolder::create(&dir).unwrap();
        assert!(!dir.join("files").exists() && !dir.join("cars").exists());
        assert!(dir.join("mod.toml").is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
