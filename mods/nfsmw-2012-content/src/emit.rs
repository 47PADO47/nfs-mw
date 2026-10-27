//! Writing the mod's generated content: one `cars/<id>.toml` per car and the car files under `files/CARS/<ID>/`, next to
//! the mod's `mod.toml`, and the `manufacturers/` of the brands (`brands.rs`). The game reads these folders after the mod
//! has loaded (`docs/modding/api-content.md`, "Cars", "Manufacturers").

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Role;
use crate::effects::EffectFiles;
use crate::stats::ImportedStats;

/// One imported car, as the mod folder records it.
#[derive(Debug, Clone, PartialEq)]
pub struct CarEntry {
    /// The car folder, upper case.
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub role: Role,
    /// The car's place in the cop waves, when it is a police car (`police.rs`): its unit class and its levels.
    pub police: Option<crate::police::Plan>,
    /// What the car is, imported from its own 2012 data (the `[stats]` table).
    pub stats: ImportedStats,
    /// The car's own paints: the `[customization]` table text, empty when the car has none (`paints::manifest_table`).
    pub customization: String,
}

/// The files of one car.
pub struct CarFiles<'a> {
    pub geometry: &'a [u8],
    pub textures: &'a [u8],
    /// The car's collision bounds, written as `BOUNDS.BIN`: a car without them cannot drive.
    pub bounds: &'a [u8],
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
    /// cleared, with `manufacturers/`. They belong to the generator; nothing else of the mod folder is touched.
    pub fn create(root: &Path) -> Result<Self> {
        for part in ["files", "cars", "manufacturers"] {
            let path = root.join(part);
            if path.exists() {
                std::fs::remove_dir_all(&path).with_context(|| format!("clearing {}", path.display()))?;
            }
        }
        std::fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
        Ok(Self { root: root.to_owned() })
    }

    /// The mod folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Add `text` (whole tables, starting with a newline) to the end of the manifest of the car `id`, which `add_car` has written.
    pub fn append_to_manifest(&self, id: &str, text: &str) -> Result<()> {
        use std::io::Write;
        let path = self.root.join("cars").join(format!("{}.toml", id.to_ascii_lowercase()));
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .with_context(|| format!("opening {}", path.display()))?;
        file.write_all(text.as_bytes()).with_context(|| format!("writing {}", path.display()))
    }

    pub fn add_car(&self, car: &CarEntry, files: &CarFiles) -> Result<()> {
        let folder = self.root.join("files").join("CARS").join(&car.id);
        std::fs::create_dir_all(&folder)?;
        std::fs::write(folder.join("GEOMETRY.BIN"), files.geometry)?;
        std::fs::write(folder.join("TEXTURES.BIN"), files.textures)?;
        std::fs::write(folder.join("BOUNDS.BIN"), files.bounds)?;
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
            let (extension, fields) = match &layer.curves {
                None => ("WAV", format!("rpm = {:?}", layer.rpm)),
                Some(curved) => (if curved.ginsu { "GIN" } else { "WAV" }, curved.fields.clone()),
            };
            std::fs::write(folder.join(format!("L{at}.{extension}")), &layer.wav)?;
            layers.push(format!("  {{ file = \"SOUND/MODENGINE/{id}/L{at}.{extension}\", {fields} }},\n"));
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
    let mut text = format!("role = {}\nname = {}\n", quoted(car.role.as_str()), quoted(&car.name));
    if !car.manufacturer.is_empty() {
        text.push_str(&format!("manufacturer = {}\n", quoted(&car.manufacturer)));
    }
    let (class, waves) = car.police.as_ref().map(|plan| plan.manifest()).unwrap_or_default();
    text.push_str(&class);
    text.push_str(&car.stats.manifest_table());
    text.push_str(&car.customization);
    text.push_str(engine.unwrap_or_default());
    // The `[[cop]]` tables go last: every key below a table header belongs to that table.
    text.push_str(&waves);
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> CarEntry {
        CarEntry {
            id: "POR911CARR12".into(),
            name: "Porsche 911 \"Carrera\" S 2012".into(),
            manufacturer: "Porsche".into(),
            role: Role::Player,
            police: None,
            stats: ImportedStats {
                mass_kg: 1397.0,
                front_weight_pct: 38.8,
                cg_height_m: 0.442,
                wheelbase_m: 2.45,
                track_m: [1.538, 1.516],
                tire_radius_m: [0.34, 0.343],
                tire_width_m: [0.246, 0.296],
                ride_height_m: 0.171,
                idle_rpm: 1000.0,
                redline_rpm: 7800.0,
                torque_curve: vec![[1000.0, 360.0], [5600.0, 440.0], [7800.0, 390.0]],
                gear_ratios: vec![3.91, 2.29, 1.55, 1.3, 1.08, 0.88, 0.71],
                reverse_ratio: 2.68,
                final_drive: 3.44,
                drive: "rwd",
                front_torque_share: 0.0,
                diff_lock: [0.0, 0.75],
                top_speed_kph: 304.2,
                tire_grip: [1.5, 1.4],
                brake_torque_nm: [1300.0, 1350.0],
                nitrous_boost: 0.9,
            },
            customization: "
[customization]
paints = [{ name = \"Red\", color = [200, 0, 0] }]
"
            .into(),
        }
    }

    #[test]
    fn the_car_manifest_is_valid_toml_the_game_accepts() {
        let text = car_manifest(&entry(), None);
        let parsed: toml::Table = toml::from_str(&text).unwrap();
        assert!(parsed.get("clone").is_none());
        assert_eq!(parsed["name"].as_str(), Some("Porsche 911 \"Carrera\" S 2012"));
        assert_eq!(parsed["role"].as_str(), Some("player"));
        assert_eq!(parsed["customization"]["paints"].as_array().map(Vec::len), Some(1));
        let stats = parsed["stats"].as_table().unwrap();
        assert_eq!(stats["mass_kg"].as_float(), Some(1397.0));
        assert_eq!(stats["drive"].as_str(), Some("rwd"));
        assert!(parsed.get("performance").is_none(), "a car has stats, not multipliers of a clone");
    }

    #[test]
    fn the_effects_table_names_every_file_it_writes_and_leaves_out_what_the_car_lacks() {
        let effects = EffectFiles { shift: vec![vec![1], vec![2]], backfire: vec![vec![3]], ..EffectFiles::default() };
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
        folder
            .add_car(&entry(), &CarFiles { geometry: b"g", textures: b"t", bounds: b"b", engine: None, effects: None })
            .unwrap();
        assert!(dir.join("files/CARS/POR911CARR12/GEOMETRY.BIN").is_file());
        assert_eq!(std::fs::read(dir.join("files/CARS/POR911CARR12/BOUNDS.BIN")).unwrap(), b"b");
        assert!(dir.join("cars/por911carr12.toml").is_file());
        ModFolder::create(&dir).unwrap();
        assert!(!dir.join("files").exists() && !dir.join("cars").exists());
        assert!(dir.join("mod.toml").is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_police_car_is_written_as_a_cop_with_its_class_and_waves_and_the_game_reads_them() {
        use crate::police::{Class, Plan, PoliceRole, Wave};
        let mut car = entry();
        car.role = Role::Cop;
        let waves = vec![Wave { heat: 6, count: 2, chance: 80 }, Wave { heat: 7, count: 6, chance: 90 }];
        car.police = Some(Plan { role: PoliceRole::Cop, class: Class::Heavy, waves });
        let text = car_manifest(&car, None);
        let manifest = blackbox_modding::content::parse_car("por911carr12", &text).unwrap();
        assert_eq!(manifest.role, blackbox_modding::content::Role::Cop);
        assert_eq!(manifest.cop_class, blackbox_modding::content::CopClass::Heavy);
        let seen: Vec<(u32, u32, u32)> = manifest.waves.iter().map(|w| (w.heat, w.count, w.chance)).collect();
        assert_eq!(seen, [(6, 2, 80), (7, 6, 90)]);
        assert!(manifest.customization.is_some(), "the car's other tables are still read");
        let plain = car_manifest(&entry(), None);
        assert!(!plain.contains("cop"), "a player car carries no police key");
    }

    #[test]
    fn the_shared_sounds_are_added_to_a_written_manifest_and_the_game_reads_them() {
        let dir = std::env::temp_dir().join(format!("nfsmw-import2012-shared-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let folder = ModFolder::create(&dir).unwrap();
        let effects = EffectFiles { shift: vec![vec![1]], ..EffectFiles::default() };
        let files = CarFiles { geometry: b"g", textures: b"t", bounds: b"b", engine: None, effects: Some(&effects) };
        folder.add_car(&entry(), &files).unwrap();
        let table = "\n[car_sound.shared]\nimpact_big = [\"SOUND/MODSHARED/IMPACT/B.WAV\"]\nscrape = [\"SOUND/MODSHARED/SCRAPE/S.WAV\"]\n";
        folder.append_to_manifest("POR911CARR12", table).unwrap();
        let text = std::fs::read_to_string(dir.join("cars/por911carr12.toml")).unwrap();
        let manifest = blackbox_modding::content::parse_car("por911carr12", &text).unwrap();
        let sound = manifest.car_sound.unwrap();
        assert_eq!(sound.shift, ["SOUND/MODENGINE/POR911CARR12/SHIFT0.WAV"], "the car's own list is kept");
        assert_eq!(sound.shared.impact_big, ["SOUND/MODSHARED/IMPACT/B.WAV"]);
        assert_eq!(sound.shared.scrape, ["SOUND/MODSHARED/SCRAPE/S.WAV"]);
        assert!(folder.append_to_manifest("NOSUCHCAR", table).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
