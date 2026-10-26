//! The settings of the nfsmw-2012-content mod: the `config.toml` in the mod's own folder.
//!
//! ```toml
//! # Folder of the user's own NFS Most Wanted (2012) install. Left out: the mod adds no cars and says why.
//! source = 'D:\Need For Speed Most Wanted 2012'
//! # Which cars: "all", or a list of ids or name fragments ("Porsche", "POR911CARR12").
//! cars = "all"
//! # Cars to leave out, by id or name fragment.
//! exclude = []
//! # Biggest texture side in pixels; larger textures drop their top mip levels.
//! max_texture_size = 1024
//! # Who can use the cars: "player", "cop" or "both".
//! role = "player"
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::performance::{Change, Performance, Tuning};

/// The file name of the settings file, in the mod's folder.
pub const FILE_NAME: &str = "config.toml";

/// A commented settings file with the defaults: what the mod writes into its folder when it finds none.
pub const DEFAULT_CONFIG: &str = "# Settings of the NFS: Most Wanted (2012) cars mod (docs/modding/nfsmw-2012-content.md). Restart the game after a change.

# Folder of your own NFS Most Wanted (2012) install. Without it no car is added.
# source = 'D:\\Need For Speed Most Wanted 2012'

# Which cars: \"all\", or a list of ids or name fragments.
cars = \"all\"
# cars = [\"Porsche\", \"Lamborghini\"]

# Cars to leave out, by id or name fragment.
exclude = []

# Longest texture side in pixels (a power of two, 16 to 4096). Bigger textures lose their top mip levels.
max_texture_size = 1024

# Who can use the cars: \"player\", \"cop\" or \"both\".
role = \"player\"

# Play the engine loops of each 2012 car (an approximation: see docs/modding/car-customization.md). The loops are
# written into the mod's folder; false leaves the cars with the game's generic engine sound.
own_engine_sound = true

# Each car drives with the stats of its own 2012 data (mass, engine torque, gearbox, tyres, brakes). These optional
# multipliers change them before they are written to the car: power scales the torque at every engine speed, mass the
# weight, grip the tyres, brakes the brake torque (0.25 to 4 each, grip 0.25 to 3), nitrous_power the extra torque of
# the nitrous (0 to 4). The 2012 cars have their nitrous from the factory; turn it off with nitrous = false.
# [performance]
# power = 1.0
# mass = 1.0
# grip = 1.0
# brakes = 1.0
# nitrous = true
# nitrous_power = 1.0

# One car or a make can differ: a table named by the car's id or a part of its name, changing only the keys it lists.
# [car.POR911CARR12]
# power = 1.15
# [car.Lamborghini]
# grip = 1.1
";

/// Who may drive or chase with the imported cars (the mod car manifest's `role`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    Player,
    Cop,
    Both,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Cop => "cop",
            Self::Both => "both",
        }
    }
}

/// Which cars to import.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Selection {
    #[default]
    All,
    Only(Vec<String>),
}

impl<'de> Deserialize<'de> for Selection {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Word(String),
            List(Vec<String>),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Word(word) if word.eq_ignore_ascii_case("all") => Ok(Self::All),
            Raw::Word(word) => Err(serde::de::Error::custom(format!("cars = {word:?}: use \"all\" or a list"))),
            Raw::List(list) => Ok(Self::Only(list)),
        }
    }
}

/// The settings, as the file holds them. Unknown keys are an error so a typo does not silently do nothing.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// The 2012 install folder.
    pub source: Option<PathBuf>,
    pub cars: Selection,
    pub exclude: Vec<String>,
    pub max_texture_size: u32,
    pub role: Role,
    /// Whether each car plays the engine loops of its 2012 bundle.
    pub own_engine_sound: bool,
    /// Multipliers on the stats imported from each car's 2012 data (`[performance]`).
    pub performance: Performance,
    /// Cars that differ from it: `[car.<id or part of the name>]`.
    pub car: BTreeMap<String, Change>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source: None,
            cars: Selection::All,
            exclude: Vec::new(),
            max_texture_size: 1024,
            role: Role::Player,
            own_engine_sound: true,
            performance: Performance::default(),
            car: BTreeMap::new(),
        }
    }
}

/// Whether `pattern` names the car with `id` and display `name`: the id itself, or a part of the name, in any case.
pub fn matches(pattern: &str, id: &str, name: &str) -> bool {
    let pattern = pattern.to_ascii_lowercase();
    id.eq_ignore_ascii_case(&pattern) || name.to_ascii_lowercase().contains(&pattern)
}

impl Config {
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
        if !self.max_texture_size.is_power_of_two() || !(16..=4096).contains(&self.max_texture_size) {
            bail!("max_texture_size {} must be a power of two from 16 to 4096", self.max_texture_size);
        }
        self.tuning().check()
    }

    /// How the cars are tuned: the general table and the cars that differ.
    pub fn tuning(&self) -> Tuning<'_> {
        Tuning { general: self.performance, cars: &self.car }
    }

    /// Whether the car with `id` and display `name` is wanted.
    pub fn wants(&self, id: &str, name: &str) -> bool {
        let matches = |pattern: &String| matches(pattern, id, name);
        if self.exclude.iter().any(matches) {
            return false;
        }
        match &self.cars {
            Selection::All => true,
            Selection::Only(list) => list.iter().any(matches),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_settings_file_parses_to_the_defaults() {
        assert_eq!(Config::parse(DEFAULT_CONFIG).unwrap(), Config::default());
    }

    #[test]
    fn defaults_want_everything() {
        let config = Config::default();
        assert!(config.wants("POR911CARR12", "Porsche 911 Carrera S 2012"));
    }

    #[test]
    fn selection_and_exclusion_match_ids_and_name_fragments() {
        let config: Config = toml::from_str("cars = ['porsche', 'LAM']\nexclude = ['GT2']").unwrap();
        assert!(config.wants("POR911CARR12", "Porsche 911 Carrera S 2012"));
        assert!(config.wants("LAMGALLARDO", "Lamborghini Gallardo 2011"));
        assert!(!config.wants("POR911GT208", "Porsche 911 GT2 2008"));
        assert!(!config.wants("BMW1M12", "BMW 1M 2012"));
    }

    #[test]
    fn bad_settings_are_refused() {
        assert!(toml::from_str::<Config>("cars = 'some'").is_err());
        assert!(toml::from_str::<Config>("typo = 1").is_err());
        let config: Config = toml::from_str("max_texture_size = 1000").unwrap();
        assert!(config.check().is_err());
    }
}
