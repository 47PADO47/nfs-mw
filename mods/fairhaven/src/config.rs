//! The settings of the native mod `fairhaven`, from the host's `bb.config` (`docs/modding/api-config.md`): the `config.toml` in
//! the mod's own folder, with the `[mods.fairhaven]` table of the game's main config file merged over it key by key. A host
//! without `bb.config` gets the file alone, read here.
//!
//! ```toml
//! # Folder of your own NFS Most Wanted (2012) install.
//! source = 'D:\Need For Speed Most Wanted 2012'
//! # Offer Fairhaven as a world to play in.
//! enabled = true
//! # The longest side of a picture, pixels.
//! max_texture = 512
//! # How bright the pictures are, percent (100 = as the 2012 game stores them).
//! brightness = 70
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// The file name of the settings file, in the mod's folder.
pub const FILE_NAME: &str = "config.toml";

/// The mod's id: its table in the game's main config file is `[mods.fairhaven]`.
pub const MOD_ID: &str = "fairhaven";

/// The brightness the pictures have by default, and the range the setting may take, percent.
pub const DEFAULT_BRIGHTNESS: u32 = 70;
pub const MIN_BRIGHTNESS: u32 = 20;
pub const MAX_BRIGHTNESS: u32 = 150;

/// The settings as the host merged them (`bb.config`): the TOML text, and which sources the host found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supplied {
    /// The merged settings as TOML text.
    pub text: String,
    /// The mod's own `config.toml` exists.
    pub has_file: bool,
    /// The game's main config has a `[mods.fairhaven]` table.
    pub has_main_section: bool,
}

impl Supplied {
    /// Whether the player has written settings anywhere. Without any, the mod writes its commented template.
    pub fn is_empty(&self) -> bool {
        !self.has_file && !self.has_main_section
    }
}

/// A commented settings file: what the mod writes into its folder when it finds no settings.
pub const DEFAULT_CONFIG: &str = "# Settings of the Fairhaven mod (mods/fairhaven/README.md). Restart the game after a change.
# The same keys work under [mods.fairhaven] in the game's own config file, which wins over this file key by key.

# Folder of your own NFS Most Wanted (2012) install (the one with HAWAII/ and SHADERS.BNDL). Without it the mod adds no world.
# source = 'D:\\Need For Speed Most Wanted 2012'

# Offer Fairhaven in the gameplay options (Free Roam World). false: the world is not offered and nothing of the install is read.
enabled = true

# Longest side of a picture in pixels (a power of two, 64 to 2048). Bigger pictures take more memory and time per unit.
max_texture = 512

# How bright the pictures are, percent of the 2012 game's pixels (20 to 150). The 2012 pictures are made for a lighting that
# darkens them (a tone map, ambient occlusion); this game's world shader multiplies a picture by twice its light, so roofs and
# roads come out near white at 100. Lower is darker.
brightness = 70
";

/// The settings of the mod.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub source: Option<PathBuf>,
    pub enabled: bool,
    pub max_texture: u32,
    pub brightness: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self { source: None, enabled: true, max_texture: 512, brightness: DEFAULT_BRIGHTNESS }
    }
}

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
        if !self.max_texture.is_power_of_two() || !(64..=2048).contains(&self.max_texture) {
            bail!("max_texture {} must be a power of two from 64 to 2048", self.max_texture);
        }
        if !(MIN_BRIGHTNESS..=MAX_BRIGHTNESS).contains(&self.brightness) {
            bail!("brightness {} must be from {MIN_BRIGHTNESS} to {MAX_BRIGHTNESS} percent", self.brightness);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fairhaven-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_template_parses_to_the_defaults_with_no_source() {
        let config = Config::parse(DEFAULT_CONFIG).unwrap();
        assert_eq!(config, Config::default());
        assert!(config.source.is_none() && config.enabled);
    }

    #[test]
    fn every_key_is_read_and_a_typo_is_refused() {
        let config =
            Config::parse("source = 'D:/mw2012'\nenabled = false\nmax_texture = 1024\nbrightness = 55\n").unwrap();
        assert_eq!(config.brightness, 55);
        assert!(Config::parse("brightness = 10").is_err());
        assert!(Config::parse("brightness = 200").is_err());
        assert_eq!(config.source.as_deref(), Some(Path::new("D:/mw2012")));
        assert!(!config.enabled);
        assert_eq!(config.max_texture, 1024);
        assert!(Config::parse("sorce = 'x'").is_err());
        assert!(Config::parse("max_texture = 1000").is_err());
        assert!(Config::parse("max_texture = 4096").is_err());
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
}
