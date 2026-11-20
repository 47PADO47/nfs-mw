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
//!
//! What the shop sold is kept in the same file, as the items the player owns (`part:id`), so wearing a bought item again costs
//! nothing:
//!
//! ```toml
//! owned = ["helmet:HL_AGV_Corsa-R_Mono-Black", "suit:SU_Alpinestars_GP-Force_Black"]
//! ```

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::catalogue::Part;

/// The table of the file.
const TABLE: &str = "gear";
/// The list of the items the profile bought.
const OWNED: &str = "owned";

/// What the profile's file holds: what the rider wears and what the player owns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kept {
    pub choice: Choice,
    pub owned: BTreeSet<String>,
}

/// How the file names an owned item: `helmet:HL_AGV_Corsa-R_Mono-Black`.
pub fn owned_key(part: Part, id: &str) -> String {
    format!("{}:{id}", part.key())
}

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

/// What the file at `path` holds (the rider's own gear and nothing owned when it is missing or does not read).
pub(super) fn read_kept(path: &Path) -> Kept {
    let Ok(text) = std::fs::read_to_string(path) else { return Kept::default() };
    let Ok(file) = text.parse::<toml::Table>() else {
        log::warn!("{}: the rider's gear does not read", path.display());
        return Kept::default();
    };
    let table = file.get(TABLE).and_then(toml::Value::as_table);
    let mut choice = Choice::default();
    for part in Part::ALL {
        let id = table.and_then(|table| table.get(part.key())).and_then(toml::Value::as_str);
        choice.set(part, id.unwrap_or_default());
    }
    let owned = file.get(OWNED).and_then(toml::Value::as_array).into_iter().flatten();
    Kept { choice, owned: owned.filter_map(|key| key.as_str().map(str::to_owned)).collect() }
}

/// The file replaced atomically, as the My Cars slots are. Parts that wear the rider's own gear are left out.
pub(super) fn write_kept(path: &Path, kept: &Kept) -> Result<()> {
    let mut gear = toml::Table::new();
    for part in Part::ALL {
        let id = kept.choice.get(part);
        if !id.is_empty() {
            gear.insert(part.key().into(), toml::Value::String(id.to_owned()));
        }
    }
    let mut file = toml::Table::new();
    file.insert(TABLE.into(), toml::Value::Table(gear));
    if !kept.owned.is_empty() {
        file.insert(OWNED.into(), toml::Value::Array(kept.owned.iter().cloned().map(toml::Value::String).collect()));
    }
    let text = toml::to_string(&file).context("Serializing the rider's gear")?;
    let parent = path.parent().context("Missing rider gear folder")?;
    std::fs::create_dir_all(parent).context("Creating the rider gear folder")?;
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, text).context("Staging the rider's gear")?;
    std::fs::rename(temporary, path).context("Replacing the rider's gear")
}
