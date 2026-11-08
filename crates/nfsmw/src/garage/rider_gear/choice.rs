//! The gear the player picked, kept beside the save of the profile.
//!
//! The rider's gear is the player's, not a part of a car: it has no price and the original game's save has no place for it, so it
//! is kept in the profile's own mod data folder (`rider-gear.toml`, next to `my-cars.toml`; a guest has `rider-gear-guest.toml` in
//! the settings folder), as the licence plate texts are (`garage/plates.rs`):
//!
//! ```toml
//! [gear]
//! helmet = "HL_AGV_Corsa-R_Mono-Black"
//! suit = "SU_Alpinestars_GP-Force_Black"
//! ```
//!
//! A part left out wears the rider's own gear (the empty id). One choice belongs to the player for every bike, since the rider
//! is the same on each. The original save is untouched.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::catalogue::Part;

/// The table of the file.
const TABLE: &str = "gear";

/// What the player wears: for each part the id of an item of the catalogue, the empty id being the rider's own gear.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Choice {
    pub body: String,
    pub helmet: String,
    pub suit: String,
    pub gloves: String,
    pub boots: String,
}

impl Choice {
    /// The rider's own gear throughout.
    pub const STOCK: Self = Self {
        body: String::new(),
        helmet: String::new(),
        suit: String::new(),
        gloves: String::new(),
        boots: String::new(),
    };

    pub fn get(&self, part: Part) -> &str {
        match part {
            Part::Body => &self.body,
            Part::Helmet => &self.helmet,
            Part::Suit => &self.suit,
            Part::Gloves => &self.gloves,
            Part::Boots => &self.boots,
        }
    }

    pub fn set(&mut self, part: Part, id: &str) {
        let slot = match part {
            Part::Body => &mut self.body,
            Part::Helmet => &mut self.helmet,
            Part::Suit => &mut self.suit,
            Part::Gloves => &mut self.gloves,
            Part::Boots => &mut self.boots,
        };
        *slot = id.to_owned();
    }

    /// Whether the rider wears his own gear throughout.
    pub fn is_stock(&self) -> bool {
        Part::ALL.into_iter().all(|part| self.get(part).is_empty())
    }
}

/// The file of the current profile, beside its My Cars slots.
pub(super) fn path() -> Option<PathBuf> {
    let slots = crate::garage::my_cars::path().ok()?;
    let name = match slots.file_name()?.to_str()? {
        "my-cars.toml" => "rider-gear.toml",
        _ => "rider-gear-guest.toml",
    };
    Some(slots.with_file_name(name))
}

/// What the file at `path` holds (the rider's own gear when it is missing or does not read).
pub(super) fn read(path: &Path) -> Choice {
    let Ok(text) = std::fs::read_to_string(path) else { return Choice::default() };
    let Ok(file) = text.parse::<toml::Table>() else {
        log::warn!("{}: the rider's gear does not read", path.display());
        return Choice::default();
    };
    let table = file.get(TABLE).and_then(toml::Value::as_table);
    let mut choice = Choice::default();
    for part in Part::ALL {
        let id = table.and_then(|table| table.get(part.key())).and_then(toml::Value::as_str);
        choice.set(part, id.unwrap_or_default());
    }
    choice
}

/// The file replaced atomically, as the My Cars slots are. Parts that wear the rider's own gear are left out.
pub(super) fn write(path: &Path, choice: &Choice) -> Result<()> {
    let mut gear = toml::Table::new();
    for part in Part::ALL {
        let id = choice.get(part);
        if !id.is_empty() {
            gear.insert(part.key().into(), toml::Value::String(id.to_owned()));
        }
    }
    let mut file = toml::Table::new();
    file.insert(TABLE.into(), toml::Value::Table(gear));
    let text = toml::to_string(&file).context("Serializing the rider's gear")?;
    let parent = path.parent().context("Missing rider gear folder")?;
    std::fs::create_dir_all(parent).context("Creating the rider gear folder")?;
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, text).context("Staging the rider's gear")?;
    std::fs::rename(temporary, path).context("Replacing the rider's gear")
}
