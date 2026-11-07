//! The settings of the native mod `ride6-bikes`, from the host's `bb.config` (`docs/modding/api-config.md`): the `config.toml`
//! in the mod's own folder, with the `[mods.ride6-bikes]` table of the game's main config file merged over it key by key.
//! A host without `bb.config` gets the file alone, read here.
//!
//! ```toml
//! # Folder of the exporter's output: one folder per bike with parts.json, glb/ and tex/ (docs/modding/ride6-bikes-batch.md).
//! source = 'C:\r6\bikes'
//! # The physics dump (default: physics, physdump or physdump-all inside `source`) and its Tire folder.
//! physics = 'C:\r6\physdump-all'
//! tyre_tables = 'C:\r6\physdump-all\resolved\plugin\Physics\Tire'
//! # The `sound` command's output folder: a bike with tables in it gets its own engine sound.
//! sound = 'C:\r6\snd-out'
//! # The exporter's rider folder (riding.r6ra, skeleton.json, meshes/, textures/; default: `rider` inside `source`).
//! rider = 'C:\r6\bikes\rider'
//! # The key of RIDE 6's pak files (64 hexadecimal digits) for the sound extraction commands; default: the built-in key.
//! # pak_key = '...'
//! # The longest side of a body texture, pixels.
//! max_texture = 2048
//! # The shop's options: the optional parts and upgrades of the bikes (docs/modding/ride6-bike-upgrades.md).
//! upgrades = true
//! bike_parts = 'C:\r6\bike-parts'
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// The file name of the settings file, in the mod's folder.
pub const FILE_NAME: &str = "config.toml";

/// The mod's id: its table in the game's main config file is `[mods.ride6-bikes]`.
pub const MOD_ID: &str = "ride6-bikes";

/// The settings as the host merged them (`bb.config`): the TOML text, and which sources the host found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supplied {
    /// The merged settings as TOML text.
    pub text: String,
    /// The mod's own `config.toml` exists.
    pub has_file: bool,
    /// The game's main config has a `[mods.ride6-bikes]` table.
    pub has_main_section: bool,
}

impl Supplied {
    /// Whether the player has written settings anywhere. Without any, the mod writes its commented template.
    pub fn is_empty(&self) -> bool {
        !self.has_file && !self.has_main_section
    }
}

/// A commented settings file: what the mod writes into its folder when it finds no settings.
pub const DEFAULT_CONFIG: &str = "# Settings of the RIDE 6 bikes mod (mods/ride6-bikes/README.md). Restart the game after a change.
# The same keys work under [mods.ride6-bikes] in the game's own config file, which wins over this file key by key.

# Folder of the exporter's output: one folder per bike, each with parts.json, glb/ and tex/ (docs/modding/ride6-bikes-batch.md).
# Without it the mod adds no bike.
# source = 'C:\\r6\\bikes'

# The physics dump (the physdump of docs/modding/ride6-bikes.md). Default: a folder called physics, physdump or physdump-all
# inside `source`.
# physics = 'C:\\r6\\physdump-all'

# The dump's Tire folder (resolved/plugin/Physics/Tire inside the dump by default).
# tyre_tables = 'C:\\r6\\physdump-all\\resolved\\plugin\\Physics\\Tire'

# Output folder of the `sound` command (docs/modding/ride6-bikes-sound.md): a bike with tables in it gets its own engine
# sound; the others play the game's built-in engine.
# sound = 'C:\\r6\\snd-out'

# The exporter's rider folder (riding.r6ra, skeleton.json, bike_skeleton.json, meshes/, textures/): the rider that sits on
# every bike. Default: a folder called `rider` inside `source`; without one the bikes are ridden by nobody.
# rider = 'C:\\r6\\bikes\\rider'

# The key of RIDE 6's pak files, 64 hexadecimal digits, which the sound extraction commands (`pak-sound`) pass to the pak
# reader. Left out, the mod uses its built-in key (docs/modding/ride6-bikes-sound.md); the environment variable R6_AES
# replaces both.
# pak_key = '0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF'

# Longest side of a body texture in pixels (a power of two, 64 to 4096); wheels get half of it and discs a quarter.
max_texture = 2048

# The shop's options (docs/modding/ride6-bike-upgrades.md): the aftermarket exhausts, rims, brake discs and bar pads of the bikes,
# and their engine, chassis and brake upgrades. `false` imports the bikes as they were, with every stock part in its solid.
upgrades = true

# The exporter's folder of the optional parts (`upgrades.py` and `option_export.py` write it; catalogue.json is in it). Default:
# a folder called `bike-parts` inside `source` or beside it. Without one the bikes get no options.
# bike_parts = 'C:\\r6work\\bike-parts'
";

/// The settings of the mod.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub source: Option<PathBuf>,
    pub physics: Option<PathBuf>,
    pub tyre_tables: Option<PathBuf>,
    pub sound: Option<PathBuf>,
    pub rider: Option<PathBuf>,
    /// The key of RIDE 6's pak files (`sound/pak_key.rs`); `None` uses the built-in one.
    pub pak_key: Option<String>,
    pub max_texture: u32,
    /// Whether the bikes get the shop's options (their optional parts and upgrades); `bike_parts` says where the parts are.
    pub upgrades: bool,
    /// The exporter's folder of the optional parts; `None`: [`Config::bike_parts_dir`] looks for it.
    pub bike_parts: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source: None,
            physics: None,
            tyre_tables: None,
            sound: None,
            rider: None,
            pak_key: None,
            max_texture: 2048,
            upgrades: true,
            bike_parts: None,
        }
    }
}

/// The folders inside `source` that may hold the physics dump, in the order they are tried.
const DUMP_NAMES: [&str; 3] = ["physics", "physdump", "physdump-all"];

impl Config {
    /// The settings to use. `supplied` is what the host merged; without it (a host with no `bb.config`) `path` is read alone.
    /// When no settings exist anywhere the commented template is written to `path`; the flag says so.
    pub fn resolve(supplied: Option<&Supplied>, path: &Path) -> Result<(Self, bool)> {
        let Some(supplied) = supplied else {
            let created = !path.exists();
            if created {
                std::fs::write(path, DEFAULT_CONFIG).with_context(|| format!("writing {}", path.display()))?;
            }
            return Ok((Self::load(path)?, created));
        };
        if supplied.is_empty() {
            std::fs::write(path, DEFAULT_CONFIG).with_context(|| format!("writing {}", path.display()))?;
            return Ok((Self::parse(DEFAULT_CONFIG)?, true));
        }
        let config = Self::parse(&supplied.text).with_context(|| {
            format!("reading the settings ({}, or [mods.{MOD_ID}] in the game's config)", path.display())
        })?;
        Ok((config, false))
    }

    /// Read the file at `path`; a missing file gives the defaults.
    pub fn load(path: &Path) -> Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
        };
        Self::parse(&text).with_context(|| format!("reading {}", path.display()))
    }

    /// Settings from the text of a settings file.
    pub fn parse(text: &str) -> Result<Self> {
        let config: Self = toml::from_str(text).context("parsing the settings")?;
        config.check()?;
        Ok(config)
    }

    pub fn check(&self) -> Result<()> {
        if !self.max_texture.is_power_of_two() || !(64..=4096).contains(&self.max_texture) {
            bail!("max_texture {} must be a power of two from 64 to 4096", self.max_texture);
        }
        if let Some(key) = &self.pak_key {
            crate::sound::pak_key::normalize(key).context("pak_key")?;
        }
        Ok(())
    }

    /// The physics dump folder: the `physics` key, else the first of [`DUMP_NAMES`] that exists inside `source`.
    pub fn physics_dir(&self, source: &Path) -> PathBuf {
        if let Some(dir) = &self.physics {
            return dir.clone();
        }
        DUMP_NAMES
            .iter()
            .map(|name| source.join(name))
            .find(|dir| dir.is_dir())
            .unwrap_or_else(|| source.join(DUMP_NAMES[0]))
    }

    /// The folder of the optional parts: none when `upgrades` is off; else the `bike_parts` key, else a folder called `bike-parts`
    /// inside `source` or beside it that holds a `catalogue.json`.
    pub fn bike_parts_dir(&self, source: &Path) -> Option<PathBuf> {
        if !self.upgrades {
            return None;
        }
        if let Some(dir) = &self.bike_parts {
            return Some(dir.clone());
        }
        let beside = source.parent().map(|parent| parent.join("bike-parts"));
        [Some(source.join("bike-parts")), beside].into_iter().flatten().find(|dir| dir.join("catalogue.json").is_file())
    }

    /// The rider's export folder: the `rider` key, else a folder called `rider` inside `source` when it exists.
    pub fn rider_dir(&self, source: &Path) -> Option<PathBuf> {
        if let Some(dir) = &self.rider {
            return Some(dir.clone());
        }
        let dir = source.join("rider");
        dir.is_dir().then_some(dir)
    }

    /// The dump's `Tire` folder: the `tyre_tables` key, else the dump's own when it exists.
    pub fn tyre_dir(&self, physics: &Path) -> Option<PathBuf> {
        if let Some(dir) = &self.tyre_tables {
            return Some(dir.clone());
        }
        let dir = physics.join("resolved").join("plugin").join("Physics").join("Tire");
        dir.is_dir().then_some(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_template_parses_to_the_defaults_with_no_source() {
        let config = Config::parse(DEFAULT_CONFIG).unwrap();
        assert_eq!(config, Config::default());
        assert!(config.source.is_none());
    }

    #[test]
    fn every_key_is_read_and_a_typo_is_refused() {
        let text = "source = 'C:/r6/bikes'\nphysics = 'C:/dump'\ntyre_tables = 'C:/dump/Tire'\nsound = 'C:/snd'\nmax_texture = 1024\n";
        let config = Config::parse(text).unwrap();
        assert_eq!(config.source.as_deref(), Some(Path::new("C:/r6/bikes")));
        assert_eq!(config.sound.as_deref(), Some(Path::new("C:/snd")));
        assert_eq!(config.max_texture, 1024);
        assert!(Config::parse("sorce = 'x'").is_err());
        assert!(Config::parse("max_texture = 1000").is_err());
        assert!(Config::parse("max_texture = 32").is_err());
    }

    #[test]
    fn the_shop_is_on_unless_switched_off_and_finds_its_parts_beside_the_source() {
        let root = temp("shop");
        let source = root.join("bikes");
        std::fs::create_dir_all(&source).unwrap();
        assert!(Config::default().upgrades);
        assert_eq!(Config::default().bike_parts_dir(&source), None, "no catalogue, no options");
        std::fs::create_dir_all(root.join("bike-parts")).unwrap();
        assert_eq!(Config::default().bike_parts_dir(&source), None, "a folder without a catalogue is not the parts");
        std::fs::write(root.join("bike-parts/catalogue.json"), "{}").unwrap();
        assert_eq!(Config::default().bike_parts_dir(&source), Some(root.join("bike-parts")));
        let off = Config::parse("upgrades = false").unwrap();
        assert_eq!(off.bike_parts_dir(&source), None);
        let named = Config::parse("bike_parts = 'C:/parts'").unwrap();
        assert_eq!(named.bike_parts_dir(&source).as_deref(), Some(Path::new("C:/parts")));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_pak_key_is_optional_and_checked() {
        let good = format!("pak_key = '{}'", "ab".repeat(32));
        assert_eq!(Config::parse(&good).unwrap().pak_key, Some("ab".repeat(32)));
        assert_eq!(Config::parse("").unwrap().pak_key, None);
        assert!(Config::parse("pak_key = 'abc'").is_err());
        assert!(Config::parse(&format!("pak_key = '{}'", "zz".repeat(32))).is_err());
    }

    #[test]
    fn the_rider_is_the_key_or_a_rider_folder_inside_the_source() {
        let source = temp("rider");
        assert_eq!(Config::default().rider_dir(&source), None);
        std::fs::create_dir_all(source.join("rider")).unwrap();
        assert_eq!(Config::default().rider_dir(&source), Some(source.join("rider")));
        let config = Config::parse("rider = 'C:/elsewhere'").unwrap();
        assert_eq!(config.rider_dir(&source).as_deref(), Some(Path::new("C:/elsewhere")));
        std::fs::remove_dir_all(&source).unwrap();
    }

    #[test]
    fn a_first_start_writes_the_template_and_a_later_one_does_not() {
        let dir = temp("first");
        let path = dir.join(FILE_NAME);
        let (config, created) = Config::resolve(None, &path).unwrap();
        assert!(created && config.source.is_none() && path.is_file());
        let (_, created) = Config::resolve(None, &path).unwrap();
        assert!(!created);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn settings_from_the_host_win_and_a_main_table_alone_writes_no_file() {
        let dir = temp("host");
        let path = dir.join(FILE_NAME);
        let supplied = Supplied { text: "source = 'X'\n".to_owned(), has_file: false, has_main_section: true };
        let (config, created) = Config::resolve(Some(&supplied), &path).unwrap();
        assert!(!created && !path.exists());
        assert_eq!(config.source.as_deref(), Some(Path::new("X")));
        let none = Supplied { text: String::new(), has_file: false, has_main_section: false };
        let (_, created) = Config::resolve(Some(&none), &path).unwrap();
        assert!(created && path.is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_dump_is_found_inside_the_source_unless_the_key_names_it() {
        let dir = temp("dump");
        std::fs::create_dir_all(dir.join("physdump")).unwrap();
        let config = Config::default();
        assert_eq!(config.physics_dir(&dir), dir.join("physdump"));
        let explicit = Config { physics: Some(PathBuf::from("Z")), ..Config::default() };
        assert_eq!(explicit.physics_dir(&dir), PathBuf::from("Z"));
        assert_eq!(config.tyre_dir(&dir.join("physdump")), None);
        std::fs::create_dir_all(dir.join("physdump/resolved/plugin/Physics/Tire")).unwrap();
        assert!(config.tyre_dir(&dir.join("physdump")).is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
