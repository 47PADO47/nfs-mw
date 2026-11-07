//! The settings of the native mod `ride6-tracks`, from the host's `bb.config` (`docs/modding/api-config.md`): the `config.toml`
//! in the mod's own folder, with the `[mods.ride6-tracks]` table of the game's main config file merged over it key by key.
//! A host without `bb.config` gets the file alone, read here.
//!
//! ```toml
//! # Folder of the exporter's track packs: one folder per track with pack.json, ground.r6m and tex/.
//! source = 'C:/r6work/tracks'
//! # The longest side of a ground picture, pixels.
//! max_texture = 512
//! # The barriers, buildings and props of the track (more than the ground alone: 190 MB for Okayama instead of 110).
//! objects = true
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// The file name of the settings file, in the mod's folder.
pub const FILE_NAME: &str = "config.toml";

/// The mod's id: its table in the game's main config file is `[mods.ride6-tracks]`.
pub const MOD_ID: &str = "ride6-tracks";

/// The settings as the host merged them (`bb.config`): the TOML text, and which sources the host found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supplied {
    /// The merged settings as TOML text.
    pub text: String,
    /// The mod's own `config.toml` exists.
    pub has_file: bool,
    /// The game's main config has a `[mods.ride6-tracks]` table.
    pub has_main_section: bool,
}

impl Supplied {
    /// Whether the player has written settings anywhere. Without any, the mod writes its commented template.
    pub fn is_empty(&self) -> bool {
        !self.has_file && !self.has_main_section
    }
}

/// A commented settings file: what the mod writes into its folder when it finds no settings.
pub const DEFAULT_CONFIG: &str =
    "# Settings of the RIDE 6 tracks mod (mods/ride6-tracks/README.md). Restart the game after a change.
# The same keys work under [mods.ride6-tracks] in the game's own config file, which wins over this file key by key.

# Folder of the exporter's track packs: one folder per track, each with pack.json, ground.r6m and tex/
# (docs/formats/ride6-track-pack.md). Without it the mod adds no world.
# source = 'C:/r6work/tracks'

# Longest side of a ground picture in pixels (a power of two, 64 to 2048). Smaller is a smaller world on disk.
max_texture = 512

# The barriers, buildings and props of the track, and the walls they make: a world without them is smaller (Okayama 110 MB
# instead of 190) and a rider can leave the track anywhere.
objects = true
";

/// The settings of the mod.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub source: Option<PathBuf>,
    pub max_texture: u32,
    /// Keep the barriers, buildings and props of the packs.
    pub objects: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { source: None, max_texture: 512, objects: true }
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-tracks-config-{name}-{}", std::process::id()));
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
        let config = Config::parse("source = 'C:/r6work/tracks'\nmax_texture = 1024\n").unwrap();
        assert_eq!(config.source.as_deref(), Some(Path::new("C:/r6work/tracks")));
        assert_eq!(config.max_texture, 1024);
        assert!(Config::parse("objects = false").is_ok_and(|c| !c.objects));
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
