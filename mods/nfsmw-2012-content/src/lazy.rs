//! Lazy files: the cars' heavy files made when the game opens them, not written by the conversion.
//!
//! A converted car is mostly four kinds of file: `GEOMETRY.BIN`, `TEXTURES.BIN` and the car's engine and effect recordings. Writing
//! them takes the disk and the first start's time, and they are only a function of the 2012 install and the settings. With the setting `stream_cars` (a gameplay option too) the
//! conversion still runs once, because the manifests (stats, paints, hinged parts) come from converting the car, but it
//! writes a small **stub** (`game_install::lazy_stub`, the text `BBLAZY1`, this mod's id, a key) in place of each of those
//! files. The host (`bb.content` 1.1, `provide_lazy_files`) asks this mod's [`Maker`] for the real bytes when the game reads a
//! stub, and the maker converts that car from its 2012 bundle then, keeping the last few cars in memory.
//!
//! Everything the maker needs is in the **recipe** the run writes next to the stamp (`lazy-cars.toml`): the install's folder,
//! the settings the files depend on and, for each car, its bundle's number, its liveries' numbers and its engine speeds. The
//! maker therefore serves the content on disk whatever the settings say now: a change of settings is converted again at the
//! next start and replaces the content (and the recipe) when it is promoted, and until then the stubs of the old content keep
//! working. The keys the stubs carry are `car/<ID>/GEOMETRY.BIN`, `car/<ID>/TEXTURES.BIN` and `sound/<ID>/<kind>/<n>`.

mod maker;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use blackbox_modding::sdk::{Ctx, Level};
use serde::Deserialize;

pub use maker::Maker;

use crate::config::{Config, MOD_ID};

/// The recipe's file name, in the mod's folder next to the import stamp.
pub const RECIPE_FILE: &str = "lazy-cars.toml";
/// The format number the recipe is written with.
const FORMAT: u32 = 1;
/// The gameplay option that makes the conversion write lazy files, and its label (a message id of the mod's catalogue).
pub const OPTION: &str = "stream_2012_cars";
pub const LABEL: &str = "nfsmw2012-stream-cars";

/// A file of a car that is made when it is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Geometry,
    Textures,
}

impl Part {
    /// The file's name in the car's folder.
    pub const fn file(self) -> &'static str {
        match self {
            Part::Geometry => "GEOMETRY.BIN",
            Part::Textures => "TEXTURES.BIN",
        }
    }
}

/// A kind of recording of a car: its engine layers, or one of the lists of its other sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    Engine,
    Shift,
    Whine,
    Blowoff,
    Backfire,
    Pops,
}

impl Sound {
    /// The name the key carries.
    pub const fn name(self) -> &'static str {
        match self {
            Sound::Engine => "engine",
            Sound::Shift => "shift",
            Sound::Whine => "whine",
            Sound::Blowoff => "blowoff",
            Sound::Backfire => "backfire",
            Sound::Pops => "pops",
        }
    }

    fn of(name: &str) -> Option<Self> {
        [Sound::Engine, Sound::Shift, Sound::Whine, Sound::Blowoff, Sound::Backfire, Sound::Pops]
            .into_iter()
            .find(|sound| sound.name() == name)
    }
}

/// What a key names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// A file of the car with this id.
    Car(String, Part),
    /// The `n`th recording of a kind of the car with this id.
    Sound(String, Sound, usize),
}

/// The key of a car's file.
pub fn car_key(id: &str, part: Part) -> String {
    format!("car/{id}/{}", part.file())
}

/// The key of the `at`th recording of `kind` of the car `id`.
pub fn sound_key(id: &str, kind: Sound, at: usize) -> String {
    format!("sound/{id}/{}/{at}", kind.name())
}

/// What `key` names, when it is a key this mod writes.
pub fn parse_key(key: &str) -> Option<Key> {
    let mut parts = key.split('/');
    let (group, id) = (parts.next()?, parts.next()?);
    if id.is_empty() {
        return None;
    }
    let parsed = match group {
        "car" => {
            let file = parts.next()?;
            let part = [Part::Geometry, Part::Textures].into_iter().find(|part| part.file() == file)?;
            Key::Car(id.to_owned(), part)
        }
        "sound" => {
            let kind = Sound::of(parts.next()?)?;
            Key::Sound(id.to_owned(), kind, parts.next()?.parse().ok()?)
        }
        _ => return None,
    };
    parts.next().is_none().then_some(parsed)
}

/// The stub that stands for the file `key` in the mod's layer.
pub fn stub(key: &str) -> Vec<u8> {
    format!("BBLAZY1\n{MOD_ID}\n{key}\n").into_bytes()
}

/// One car of the recipe: what the maker needs to convert it again.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CarRecipe {
    pub id: String,
    /// The number of the car's bundle (`VEH_<number>_HI.BNDL`).
    pub file: u32,
    /// The numbers of the bundles of its liveries.
    #[serde(default)]
    pub liveries: Vec<u32>,
    /// The engine speeds its layers were made for.
    pub idle_rpm: f32,
    pub redline_rpm: f32,
}

/// What the lazy files of a conversion run are made from.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Recipe {
    format: u32,
    /// The 2012 install the files are made from.
    pub source: PathBuf,
    pub max_texture_size: u32,
    pub driver_poses: bool,
    #[serde(default)]
    pub car: Vec<CarRecipe>,
}

impl Recipe {
    /// The recipe of a run with `config` over the install at `source`, for `cars`.
    pub fn new(config: &Config, source: &Path, mut cars: Vec<CarRecipe>) -> Self {
        cars.sort_by(|a, b| a.id.cmp(&b.id));
        let source = std::path::absolute(source).unwrap_or_else(|_| source.to_owned());
        Self {
            format: FORMAT,
            source,
            max_texture_size: config.max_texture_size,
            driver_poses: config.driver_poses,
            car: cars,
        }
    }

    /// The recipe as the text of its file.
    pub fn text(&self) -> String {
        let quoted = |text: &str| format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""));
        let numbers = |list: &[u32]| list.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
        let mut text = format!(
            "# What the lazy files of nfsmw-2012-content are made from (lazy.rs). Written by the conversion; do not edit.\n\
             format = {}\nsource = {}\nmax_texture_size = {}\ndriver_poses = {}\n",
            self.format,
            quoted(&self.source.to_string_lossy()),
            self.max_texture_size,
            self.driver_poses
        );
        for car in &self.car {
            text.push_str(&format!(
                "\n[[car]]\nid = {}\nfile = {}\nliveries = [{}]\nidle_rpm = {:?}\nredline_rpm = {:?}\n",
                quoted(&car.id),
                car.file,
                numbers(&car.liveries),
                car.idle_rpm,
                car.redline_rpm
            ));
        }
        text
    }

    /// Read a recipe from its text. A recipe of another format is an error.
    pub fn parse(text: &str) -> Result<Self> {
        let recipe: Self = toml::from_str(text).context("parsing the recipe of the lazy files")?;
        anyhow::ensure!(
            recipe.format == FORMAT,
            "the recipe of the lazy files has format {}, not {FORMAT}",
            recipe.format
        );
        Ok(recipe)
    }

    /// The car with this id (case does not matter).
    pub fn car(&self, id: &str) -> Option<&CarRecipe> {
        self.car.iter().find(|car| car.id.eq_ignore_ascii_case(id))
    }

    /// Write the recipe into the mod folder `dir` (or the staging folder that becomes it).
    pub fn write(&self, dir: &Path) -> Result<()> {
        let path = dir.join(RECIPE_FILE);
        std::fs::write(&path, self.text()).with_context(|| format!("writing {}", path.display()))
    }

    /// The recipe in the mod folder `dir`, when it has one that reads.
    pub fn read(dir: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(dir.join(RECIPE_FILE)).ok()?;
        Self::parse(&text).map_err(|error| log::warn!("{error:#}")).ok()
    }
}

/// Decide whether a conversion with `config` writes lazy files, and offer the gameplay option **Stream 2012 cars** for it.
/// They are written when the setting `stream_cars` is on, the host can make them (`bb.content` 1.1) and the option is on
/// (it is on until the player turns it off). The option is offered when a 2012 install is set; a change of it is converted at
/// the next start. A host without `bb.settings` has the setting alone.
pub fn decide(ctx: &Ctx, config: &Config) -> bool {
    let can_make = ctx.content().is_some_and(|content| content.supports_lazy_files());
    if !can_make || !config.stream_cars || config.source.is_none() {
        return false;
    }
    let Some(settings) = ctx.settings() else { return true };
    if let Err(error) = settings.section(crate::SECTION).add_toggle(OPTION, LABEL, true, |_ctx, _on| {}) {
        ctx.log(Level::Warn, &format!("nfsmw-2012-content: the stream setting is not added: {error}"));
        return true;
    }
    match settings.get(OPTION) {
        Ok(value) => value == "on",
        Err(_) => true,
    }
}

/// Give the host the maker of the lazy files in the mod folder `mod_dir`, when it has a recipe: the files its stubs stand for
/// are then made when the game reads them. Called at load for content already there, and again when a first run is promoted.
pub fn provide(ctx: &Ctx, mod_dir: &Path) {
    let roads = crate::road_textures::Maker::read(mod_dir);
    let recipe = match (Recipe::read(mod_dir), &roads) {
        (Some(recipe), _) => recipe,
        // No cars are streamed, but the road textures are: the maker has no car to make.
        (None, Some(roads)) => Recipe::new(&Config::default(), roads.source(), Vec::new()),
        (None, None) => return,
    };
    let Some(content) = ctx.content().filter(|content| content.supports_lazy_files()) else {
        let reason = "nfsmw-2012-content: the cars are lazy files and this game cannot make them: reconvert with stream_cars = false";
        ctx.log(Level::Warn, reason);
        return;
    };
    let cars = recipe.car.len();
    match content.provide_lazy_files(Maker::new(recipe).with_roads(roads)) {
        Ok(()) => {
            let said = format!(
                "nfsmw-2012-content: the files of {cars} cars and the road textures are made when the game opens them"
            );
            ctx.log(Level::Info, &said);
        }
        Err(error) => ctx.log(Level::Warn, &format!("nfsmw-2012-content: the lazy files are not offered: {error}")),
    }
}
