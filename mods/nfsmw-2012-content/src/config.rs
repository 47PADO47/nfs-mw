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

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

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
}

impl Default for Config {
    fn default() -> Self {
        Self { source: None, cars: Selection::All, exclude: Vec::new(), max_texture_size: 1024, role: Role::Player }
    }
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
        Ok(())
    }

    /// Whether the car with `id` and display `name` is wanted.
    pub fn wants(&self, id: &str, name: &str) -> bool {
        let matches = |pattern: &String| {
            let pattern = pattern.to_ascii_lowercase();
            let (id, name) = (id.to_ascii_lowercase(), name.to_ascii_lowercase());
            id == pattern || name.contains(&pattern)
        };
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
