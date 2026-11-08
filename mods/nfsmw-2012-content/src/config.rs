//! The settings of the nfsmw-2012-content mod. They come from the host's `bb.config` (docs/modding/api-config.md): the
//! `config.toml` in the mod's own folder, with the `[mods.nfsmw-2012-content]` table of the game's main config file merged
//! over it key by key. The same keys work in either place. A host without `bb.config` gets the file alone, read here.
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
use crate::police::Police;

/// The file name of the settings file, in the mod's folder.
pub const FILE_NAME: &str = "config.toml";

/// The mod's id: its table in the game's main config file is `[mods.nfsmw-2012-content]`.
pub const MOD_ID: &str = "nfsmw-2012-content";

/// The settings as the host merged them (`bb.config`): the TOML text, and which sources the host found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supplied {
    /// The merged settings as TOML text.
    pub text: String,
    /// The mod's own `config.toml` exists.
    pub has_file: bool,
    /// The game's main config has a `[mods.nfsmw-2012-content]` table.
    pub has_main_section: bool,
}

impl Supplied {
    /// Whether the player has written settings anywhere. Without any, the mod writes its commented template.
    pub fn is_empty(&self) -> bool {
        !self.has_file && !self.has_main_section
    }
}

/// A commented settings file with the defaults: what the mod writes into its folder when it finds none.
pub const DEFAULT_CONFIG: &str = "# Settings of the NFS: Most Wanted (2012) cars mod (mods/nfsmw-2012-content/README.md). Restart the game after a change.
# The same keys work under [mods.nfsmw-2012-content] in the game's own config file, which wins over this file key by key.

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

# Give each 2012 car its own recordings: the engine loops of its bundle, and its gear shifts, turbo whistle and
# blow-off, backfires and pops (an approximation: see docs/modding/car-sound.md). They are written into the
# mod's folder. false gives the cars no recordings: they play the game's built-in engine and have no other car sounds.
own_engine_sound = true

# Offer the 2012 game's soundtrack (43 songs, read from your 2012 install while they play: nothing is converted or copied) as the
# gameplay option \"2012 soundtrack\", which is off until you turn it on. false adds neither the songs nor the option. Changing it
# never converts the cars again (mods/nfsmw-2012-content/README.md, \"The soundtrack\").
music = true

# Pose each car's driver for the steering and the look back (seven steering poses and one reverse pose of the figure, in the car's
# files: about 1.7 MB a car, 110 MB over 67 cars, and nothing on disk with stream_cars). false keeps the driver seated and still
# (mods/nfsmw-2012-content/README.md, \"The driver's poses\").
driver_poses = true

# Make the cars' heavy files (GEOMETRY.BIN, TEXTURES.BIN, the engine and effect recordings) when the game opens a car instead of
# writing them: the mod's folder then holds small stubs and a recipe, and each car is converted from your 2012 install the first
# time the game needs it (a moment, then kept in memory). false writes every file, which takes the disk space and the first start's
# time. Needs a game with bb.content 1.1; the gameplay option \"Stream 2012 cars\" switches it too. A change converts again at the
# next start (mods/nfsmw-2012-content/README.md, \"Streamed cars\").
stream_cars = true

# Add the 2012 game's eleven traffic cars (Audi A3, Dodge Ram 1500, GMC Topkick, ...) to the roads, among the game's own
# traffic. They are cars of their own (ids ending in _TR) that nobody drives. false adds none. The cars, exclude and
# max_texture_size keys apply to them (the name of each has \"traffic\" in it).
traffic = true

# Add the 2012 game's trees as a value of the gameplay setting \"Trees\": the models and textures of its trees (broadleaf, poplar,
# spruce, autumn and bare trees, bushes) replace those of the 2005 game's tree scenery while the setting says so. false adds
# neither the trees nor the value. They are converted from your 2012 install into the mod's folder
# (mods/nfsmw-2012-content/README.md, \"Trees\"); the max_texture_size key applies to their textures.
trees = true

# Offer the 2012 game's road, ground and sign textures (asphalt, sidewalks, grass, concrete, road paint, the STOP and speed limit
# signs, the storm grate and the round cover) as a value of the gameplay setting \"Road textures\". Nothing is converted or
# written: the pictures are made from your 2012 install when the game first loads a map tile that uses them
# (mods/nfsmw-2012-content/README.md, \"Road textures\"). false adds neither the textures nor the value.
road_textures = true

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

# The police cars of the 2012 game (the Crown Victoria and Charger cops, the Explorer Interceptor, the Corvette Z06 and the SWAT
# van: read from their bundles) are added as police cars: they chase you in the cop waves and are not in your garage. Their
# levels follow the kind of car (mods/nfsmw-2012-content/README.md, \"Police cars\"). Every key is optional:
# [police]
# auto = true                 # false: no car is a police car unless `add` names it
# add = [\"Camaro\"]           # more cars to make police cars (ids or name fragments)
# remove = [\"Explorer\"]       # cars that stay player cars although the 2012 game made them police cars
# role = \"cop\"               # \"both\": the player may drive the police cars as well
# One police car or a make: class (\"patrol\", \"pursuit\", \"heavy\") and its levels, as `heat = [..]` (2 cars at 10 percent
# each) or one entry per level with its numbers (count 1 to 8, chance 1 to 100 percent):
# [police.car.SWAT]
# class = \"heavy\"
# cop = [{ heat = 8, count = 1, chance = 30 }, { heat = 10, count = 3, chance = 60 }]
# [police.car.Crown]
# heat = [1, 2]

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
    /// Whether each car gets the recordings of its 2012 bundle (engine loops, shifts, turbo, backfire, pops).
    pub own_engine_sound: bool,
    /// Whether the 2012 game's songs are offered to the custom music player (`soundtrack.rs`). Not part of the conversion's
    /// fingerprint: the songs are streamed from the install, so changing it converts nothing.
    pub music: bool,
    /// Whether each car's driver is posed for the steering and the look back (`driver_poses.rs`).
    pub driver_poses: bool,
    /// Whether the cars' heavy files are made when the game opens them instead of being written (`lazy.rs`).
    pub stream_cars: bool,
    /// Whether this conversion writes lazy files: `stream_cars`, a host that can make them and the gameplay option on. Decided at
    /// every start (`lazy::decide`) and never read from a file; it is part of the import stamp.
    #[serde(skip)]
    pub lazy: bool,
    /// Whether the 2012 game's traffic cars are added as traffic cars (`traffic.rs`).
    pub traffic: bool,
    /// Whether the 2012 game's trees are converted and offered as a trees variant (`trees.rs`).
    pub trees: bool,
    /// Whether the 2012 game's road, ground and sign textures are offered as a replacement of the 2005 world's (`road_textures/`). Not part
    /// of the conversion's fingerprint: the set is only stubs, written at every start.
    pub road_textures: bool,
    /// Multipliers on the stats imported from each car's 2012 data (`[performance]`).
    pub performance: Performance,
    /// Cars that differ from it: `[car.<id or part of the name>]`.
    pub car: BTreeMap<String, Change>,
    /// Which cars are police cars and where they appear (`[police]`).
    pub police: Police,
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
            music: true,
            driver_poses: true,
            stream_cars: true,
            lazy: false,
            traffic: true,
            trees: true,
            road_textures: true,
            performance: Performance::default(),
            car: BTreeMap::new(),
            police: Police::default(),
        }
    }
}

/// Whether `pattern` names the car with `id` and display `name`: the id itself, or a part of the name, in any case.
pub fn matches(pattern: &str, id: &str, name: &str) -> bool {
    let pattern = pattern.to_ascii_lowercase();
    id.eq_ignore_ascii_case(&pattern) || name.to_ascii_lowercase().contains(&pattern)
}

impl Config {
    /// The settings for a start: the ones the host merged (`supplied`), or, for a host without `bb.config`, the file at
    /// `path`. When no settings exist anywhere (no file, no `[mods.nfsmw-2012-content]` table) the commented template
    /// [`DEFAULT_CONFIG`] is written to `path` first; `created` says so. A table in the main config and no file writes
    /// nothing: the file would only shadow nothing and confuse.
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
        if !self.max_texture_size.is_power_of_two() || !(16..=4096).contains(&self.max_texture_size) {
            bail!("max_texture_size {} must be a power of two from 16 to 4096", self.max_texture_size);
        }
        self.police.check().map_err(|message| anyhow::anyhow!(message))?;
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
    fn the_trees_are_on_by_default_and_the_setting_turns_them_off() {
        assert!(Config::default().trees);
        assert!(!Config::parse("trees = false\n").unwrap().trees);
        assert_ne!(
            crate::stamp::of(&Config::default(), Path::new("definitely/not/here")),
            crate::stamp::of(&Config { trees: false, ..Config::default() }, Path::new("definitely/not/here")),
            "a change of the setting converts again"
        );
    }

    #[test]
    fn selection_and_exclusion_match_ids_and_name_fragments() {
        let config: Config = toml::from_str("cars = ['porsche', 'LAM']\nexclude = ['GT2']").unwrap();
        assert!(config.wants("POR911CARR12", "Porsche 911 Carrera S 2012"));
        assert!(config.wants("LAMGALLARDO", "Lamborghini Gallardo 2011"));
        assert!(!config.wants("POR911GT208", "Porsche 911 GT2 2008"));
        assert!(!config.wants("BMW1M12", "BMW 1M 2012"));
    }

    fn supplied(text: &str, has_file: bool, has_main_section: bool) -> Supplied {
        Supplied { text: text.to_owned(), has_file, has_main_section }
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nfsmw-2012-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_hosts_merged_text_is_the_settings() {
        let dir = scratch("merged");
        let path = dir.join(FILE_NAME);
        let (config, created) =
            Config::resolve(Some(&supplied("role = 'cop'\nmax_texture_size = 512\n", false, true)), &path).unwrap();
        assert_eq!((config.role, config.max_texture_size, created), (Role::Cop, 512, false));
        assert!(!path.exists(), "a table in the main config and no file: no template is written");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn with_no_settings_anywhere_the_template_is_written_and_means_the_defaults() {
        let dir = scratch("template");
        let path = dir.join(FILE_NAME);
        let (config, created) = Config::resolve(Some(&supplied("", false, false)), &path).unwrap();
        assert_eq!((config, created), (Config::default(), true));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), DEFAULT_CONFIG);
        let (_, again) = Config::resolve(Some(&supplied("", true, false)), &path).unwrap();
        assert!(!again, "once the file exists nothing is written");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_host_without_the_interface_gets_the_file_alone() {
        let dir = scratch("legacy");
        let path = dir.join(FILE_NAME);
        let (config, created) = Config::resolve(None, &path).unwrap();
        assert_eq!((config, created), (Config::default(), true));
        std::fs::write(&path, "role = 'both'\n").unwrap();
        let (config, created) = Config::resolve(None, &path).unwrap();
        assert_eq!((config.role, created), (Role::Both, false));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_unknown_key_in_either_source_is_an_error() {
        let dir = scratch("typo");
        let path = dir.join(FILE_NAME);
        let error = Config::resolve(Some(&supplied("typo = 1\n", false, true)), &path).unwrap_err();
        assert!(format!("{error:#}").contains("typo"), "{error:#}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_change_in_either_source_changes_the_import_stamp() {
        use blackbox_modding::host::config::effective;
        let nowhere = Path::new("definitely/not/here");
        let stamp = |file: &str, main: &str| {
            let merged = effective(Some(file), Some(main)).unwrap();
            crate::stamp::of(&Config::parse(&merged.text).unwrap(), nowhere)
        };
        let base = stamp("role = 'player'\n", "max_texture_size = 1024\n");
        assert_ne!(base, stamp("role = 'cop'\n", "max_texture_size = 1024\n"), "the file changed");
        assert_ne!(base, stamp("role = 'player'\n", "max_texture_size = 512\n"), "the main config changed");
        assert_eq!(base, stamp("role = 'player'\n", "max_texture_size = 1024\n"), "nothing changed");
        assert_eq!(
            base,
            stamp("role = 'player'\nmax_texture_size = 1024\n", "role = 'player'\n"),
            "the same effective settings from other sources are the same stamp"
        );
    }

    #[test]
    fn bad_settings_are_refused() {
        assert!(toml::from_str::<Config>("cars = 'some'").is_err());
        assert!(toml::from_str::<Config>("typo = 1").is_err());
        let config: Config = toml::from_str("max_texture_size = 1000").unwrap();
        assert!(config.check().is_err());
    }

    #[test]
    fn the_drivers_are_posed_by_default_and_the_setting_turns_the_poses_off() {
        assert!(Config::default().driver_poses);
        assert!(!Config::parse("driver_poses = false\n").unwrap().driver_poses);
    }

    #[test]
    fn the_cars_stream_by_default_and_the_setting_and_the_decision_change_the_stamp() {
        assert!(Config::default().stream_cars && !Config::default().lazy);
        assert!(!Config::parse("stream_cars = false\n").unwrap().stream_cars);
        assert!(toml::from_str::<Config>("lazy = true").is_err(), "the decision is not a setting");
        let nowhere = Path::new("definitely/not/here");
        let lazy = Config { lazy: true, ..Config::default() };
        assert_ne!(
            crate::stamp::of(&Config::default(), nowhere),
            crate::stamp::of(&lazy, nowhere),
            "stubs and files are different content"
        );
    }
}
