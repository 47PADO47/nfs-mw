//! The presets of docs/specs/ac-handling/parameter-mapping.md section 5: one per tyre compound class and one modifier set per
//! car class, shipped in `presets/*.toml` and embedded in the library so the mod works with an empty `presets/` folder.
//!
//! A file that fails to parse or to validate is rejected whole with a one-line reason, and the built-in copy of that file is
//! used instead; the layer never goes missing.

mod car_file;
mod parse;
mod units;

use std::path::Path;

pub use car_file::{CarFile, parse_car_file};
pub use parse::{Preset, parse_preset};
pub use units::{UnitDefaults, UnitsTable};

use crate::keys::KeyTable;

/// The tyre compound classes, from the economy tyre to the slick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Compound {
    /// A budget road tyre.
    Economy,
    /// A road tyre.
    Road,
    /// A sport or summer tyre.
    Sport,
    /// A semi-slick.
    Semislick,
    /// A slick.
    Slick,
}

impl Compound {
    /// Every compound, in the order of the presets.
    pub const ALL: [Compound; 5] =
        [Compound::Economy, Compound::Road, Compound::Sport, Compound::Semislick, Compound::Slick];

    /// The name used in files, settings and commands.
    pub fn name(self) -> &'static str {
        match self {
            Compound::Economy => "economy",
            Compound::Road => "road",
            Compound::Sport => "sport",
            Compound::Semislick => "semislick",
            Compound::Slick => "slick",
        }
    }

    /// The compound a name says.
    pub fn parse(name: &str) -> Option<Compound> {
        Compound::ALL.into_iter().find(|compound| compound.name() == name)
    }
}

/// The car classes that carry the part of the feel that depends on the car's size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CarClass {
    /// Light, small tyres.
    Compact,
    /// The default.
    Sport,
    /// Wide, stiff tyres.
    Super,
    /// Heavy.
    Heavy,
}

impl CarClass {
    /// Every class, in the order of the presets.
    pub const ALL: [CarClass; 4] = [CarClass::Compact, CarClass::Sport, CarClass::Super, CarClass::Heavy];

    /// The name used in files and logs.
    pub fn name(self) -> &'static str {
        match self {
            CarClass::Compact => "compact",
            CarClass::Sport => "sport",
            CarClass::Super => "super",
            CarClass::Heavy => "heavy",
        }
    }

    /// The class a name says.
    pub fn parse(name: &str) -> Option<CarClass> {
        CarClass::ALL.into_iter().find(|class| class.name() == name)
    }
}

/// A compound preset: the keys that differ by compound, and how its grip follows the car's grip level.
#[derive(Debug, Clone, PartialEq)]
pub struct CompoundPreset {
    /// Which compound.
    pub compound: Compound,
    /// The grip level at which `mu_y0` is stated.
    pub level_ref: f64,
    /// `mu_y0` per unit of grip level away from `level_ref`.
    pub grip_slope: f64,
    /// The keys the preset sets.
    pub tyre: KeyTable,
    /// Which optional units start on for a car on this compound.
    pub units: UnitsTable,
}

/// A car class preset: `value = value * mul + add` for the keys it names.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassPreset {
    /// Which class.
    pub class: CarClass,
    /// Multipliers.
    pub mul: KeyTable,
    /// Additions, after the multipliers.
    pub add: KeyTable,
}

/// The five compound presets and the four class presets.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetSet {
    compounds: Vec<CompoundPreset>,
    classes: Vec<ClassPreset>,
}

/// The shipped files, embedded: name and text.
const EMBEDDED: [(&str, &str); 9] = [
    ("compound-economy.toml", include_str!("../../presets/compound-economy.toml")),
    ("compound-road.toml", include_str!("../../presets/compound-road.toml")),
    ("compound-sport.toml", include_str!("../../presets/compound-sport.toml")),
    ("compound-semislick.toml", include_str!("../../presets/compound-semislick.toml")),
    ("compound-slick.toml", include_str!("../../presets/compound-slick.toml")),
    ("class-compact.toml", include_str!("../../presets/class-compact.toml")),
    ("class-sport.toml", include_str!("../../presets/class-sport.toml")),
    ("class-super.toml", include_str!("../../presets/class-super.toml")),
    ("class-heavy.toml", include_str!("../../presets/class-heavy.toml")),
];

impl PresetSet {
    /// The presets embedded in the library. An error only if a shipped file is broken, which the tests rule out.
    pub fn builtin() -> Result<PresetSet, String> {
        PresetSet::assemble(EMBEDDED.iter().map(|(file, text)| (*file, (*text).to_owned())))
    }

    /// The presets of the folder `dir` (a mod's `presets/`), each file that is missing or rejected replaced by its built-in
    /// copy. The strings are the one-line reasons of the rejected files, for the log.
    pub fn load(dir: &Path) -> Result<(PresetSet, Vec<String>), String> {
        let mut notes = Vec::new();
        let mut texts = Vec::new();
        for (file, embedded) in EMBEDDED {
            let text = match std::fs::read_to_string(dir.join(file)) {
                Ok(text) => text,
                Err(_) => {
                    texts.push((file, embedded.to_owned()));
                    continue;
                }
            };
            match parse_preset(file, &text) {
                Ok(_) => texts.push((file, text)),
                Err(reason) => {
                    notes.push(format!("presets/{file}: {reason}; using the built-in copy"));
                    texts.push((file, embedded.to_owned()));
                }
            }
        }
        Ok((PresetSet::assemble(texts.into_iter())?, notes))
    }

    fn assemble<'a>(files: impl Iterator<Item = (&'a str, String)>) -> Result<PresetSet, String> {
        let mut compounds = Vec::new();
        let mut classes = Vec::new();
        for (file, text) in files {
            match parse_preset(file, &text)? {
                Preset::Compound(preset) => compounds.push(*preset),
                Preset::Class(preset) => classes.push(*preset),
            }
        }
        compounds.sort_by_key(|preset| Compound::ALL.iter().position(|c| *c == preset.compound));
        classes.sort_by_key(|preset| CarClass::ALL.iter().position(|c| *c == preset.class));
        if compounds.len() != Compound::ALL.len() || classes.len() != CarClass::ALL.len() {
            return Err("the preset files do not cover every compound and class".to_owned());
        }
        Ok(PresetSet { compounds, classes })
    }

    /// The preset of `compound`.
    pub fn compound(&self, compound: Compound) -> &CompoundPreset {
        let index = Compound::ALL.iter().position(|c| *c == compound).unwrap_or(0);
        &self.compounds[index]
    }

    /// The preset of `class`.
    pub fn class(&self, class: CarClass) -> &ClassPreset {
        let index = CarClass::ALL.iter().position(|c| *c == class).unwrap_or(0);
        &self.classes[index]
    }
}
