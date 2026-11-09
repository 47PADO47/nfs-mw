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
# Add the 2012 game's road signs (stop, speed limit 45 and 55, do not enter, tow zone) and traffic light models (the mast arm and
# the three-lens signal head) as values of the gameplay settings \"Road signs\" and \"Traffic light models\": they replace the
# 2005 game's solids of the same signs and lights while the settings say so. false adds neither the models nor the values. They are
# converted from your 2012 install into the mod's folder (mods/nfsmw-2012-content/README.md, \"Road signs and traffic lights\");
# the max_texture_size key applies to their textures.
signs = true
# Add the 2012 game's bushes and hedges as a value of the gameplay setting \"Bushes\": the shrubs, hedges and bush patches of the 2005 world's
# scenery (about 480 solids) are drawn with the 2012 game's shrub models while the setting says so. They come from the same
# conversion as the trees (same pictures, a few hundred KB more), so they need `trees = true`. false adds no Bushes setting and
# leaves the three bush families of the old Trees value (round bushes) in it as they were.
bushes = true
# Add the 2012 game's effects as values of the gameplay settings \"Skid marks variant\", \"Rain splash variant\" and
# \"Wet road variant\", and a replacement set for the lens drops (the kind \"Rain drops\"): the tyre marks, the water splash and
# the drop shapes are converted from your 2012 install into the mod's folder, a few KiB of it, read by ranges
# (mods/nfsmw-2012-content/README.md, \"Skid marks and rain\"). false adds neither the files nor the value.
skid_marks = true
rain_splash = true
rain_drops = true
wet_road = true
# Add the 2012 game's sky (cloud domes for the hours and the weather, the moon) as a value of the gameplay setting \"Sky\", read from
# your 2012 install into the mod's folder (about 5.5 MB; README, \"Sky\"). false adds neither. sky_size is the widest a dome picture
# is written, 256 to 2048: 2048 keeps the install's pictures, 1024 halves them and the disk they take.
sky = true
sky_size = 2048

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
    /// Whether the 2012 game's road signs and traffic light models are converted and offered as variants (`signs.rs`).
    pub signs: bool,
    /// Whether the 2012 game's bushes and hedges are offered as their own variant (`trees/bushes.rs`); needs `trees`.
    pub bushes: bool,
    /// Whether the 2012 game's skid marks, rain splash, lens drops and wet road look are converted and offered (`marks_assets.rs`,
    /// `rain_assets.rs`).
    pub skid_marks: bool,
    pub rain_splash: bool,
    pub rain_drops: bool,
    pub wet_road: bool,
    /// Whether the 2012 game's sky is converted and offered as a sky variant (`sky.rs`).
    pub sky: bool,
    /// The widest a sky dome picture is written (256 to 2048): wider ones are halved.
    pub sky_size: u32,
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
            signs: true,
            bushes: true,
            skid_marks: true,
            rain_splash: true,
            rain_drops: true,
            wet_road: true,
            sky: true,
            sky_size: 2048,
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
        if !self.sky_size.is_power_of_two() || !(256..=2048).contains(&self.sky_size) {
            bail!("sky_size {} must be a power of two from 256 to 2048", self.sky_size);
        }
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
mod tests;
