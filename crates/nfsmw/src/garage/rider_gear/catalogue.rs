//! The rider's gear catalogue as a mod ships it: `RIDER/GEAR/gear.json` of the mod's `files/` (`docs/formats/ride6-rider-gear.md`).
//! Nothing here names a game: a mod lists bodies, helmets, suits, gloves and boots with the files that make them, and the garage
//! and the rider read the list.

use anyhow::{Context, Result, ensure};
use game_install::GameDir;
use serde::Deserialize;

/// The catalogue's place in the install's virtual file system.
pub const FILE: &str = "RIDER/GEAR/gear.json";
/// The folder the item files are named from.
pub const FOLDER: &str = "RIDER/GEAR";

/// What a piece of gear is for: one list of the rider editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Part {
    Body,
    Helmet,
    Suit,
    Gloves,
    Boots,
}

impl Part {
    pub const ALL: [Self; 5] = [Self::Body, Self::Helmet, Self::Suit, Self::Gloves, Self::Boots];

    /// The name the garage page and the profile file give it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Body => "Rider Body",
            Self::Helmet => "Rider Helmet",
            Self::Suit => "Rider Suit",
            Self::Gloves => "Rider Gloves",
            Self::Boots => "Rider Boots",
        }
    }

    /// The key of the profile file's `[gear]` table and of `--part rider-<key>=`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Helmet => "helmet",
            Self::Suit => "suit",
            Self::Gloves => "gloves",
            Self::Boots => "boots",
        }
    }

    /// The part a key names.
    pub fn of_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|part| part.key() == key)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Body {
    pub id: String,
    pub name: String,
    pub mesh: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Visor {
    pub mesh: String,
    pub color: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Helmet {
    pub id: String,
    pub name: String,
    pub mesh: String,
    pub color: String,
    pub visor: Option<Visor>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Suit {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Gloves {
    pub id: String,
    pub name: String,
    pub rgb: [u8; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct Boots {
    pub id: String,
    pub name: String,
    pub mesh: String,
    pub color: String,
}

/// The meshes of the rider's own gear that an item of each part takes the place of (the names they have in his pack).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Stock {
    #[serde(default)]
    pub body: Vec<String>,
    #[serde(default)]
    pub helmet: Vec<String>,
    #[serde(default)]
    pub visor: Vec<String>,
    #[serde(default)]
    pub boots: Vec<String>,
}

/// One line of a list the garage shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// What the player's choice stores; empty for the rider's own gear.
    pub id: String,
    pub label: String,
}

/// Every item a mod offers.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Catalogue {
    pub version: u32,
    #[serde(default)]
    pub stock: Stock,
    #[serde(default)]
    pub bodies: Vec<Body>,
    #[serde(default)]
    pub helmets: Vec<Helmet>,
    #[serde(default)]
    pub suits: Vec<Suit>,
    #[serde(default)]
    pub gloves: Vec<Gloves>,
    #[serde(default)]
    pub boots: Vec<Boots>,
}

/// The label of the entry that leaves the rider his own gear.
pub const STOCK: &str = "Stock";

impl Catalogue {
    /// Reads a `gear.json`.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let catalogue: Self = serde_json::from_slice(bytes).context("gear.json")?;
        ensure!(catalogue.version == 1, "gear.json has version {}, this game reads 1", catalogue.version);
        Ok(catalogue)
    }

    /// The catalogue of the install, `None` when no mod has one or it does not read (the log says so).
    pub fn load(dir: &GameDir) -> Option<Self> {
        if !dir.exists(FILE) {
            return None;
        }
        let parsed = dir.read(FILE).map_err(anyhow::Error::from).and_then(|bytes| Self::parse(&bytes));
        match parsed {
            Ok(catalogue) => Some(catalogue),
            Err(error) => {
                log::warn!("rider gear: {FILE} is not used: {error:#}");
                None
            }
        }
    }

    /// The ids and names of a part's list, the stock entry first.
    pub fn entries(&self, part: Part) -> Vec<Entry> {
        let stock = Entry { id: String::new(), label: STOCK.to_owned() };
        let listed: Vec<Entry> = match part {
            Part::Body => self.bodies.iter().map(|i| (&i.id, &i.name)).map(entry).collect(),
            Part::Helmet => self.helmets.iter().map(|i| (&i.id, &i.name)).map(entry).collect(),
            Part::Suit => self.suits.iter().map(|i| (&i.id, &i.name)).map(entry).collect(),
            Part::Gloves => self.gloves.iter().map(|i| (&i.id, &i.name)).map(entry).collect(),
            Part::Boots => self.boots.iter().map(|i| (&i.id, &i.name)).map(entry).collect(),
        };
        std::iter::once(stock).chain(listed).collect()
    }

    /// Whether the catalogue has anything to choose from.
    pub fn is_empty(&self) -> bool {
        Part::ALL.into_iter().all(|part| self.entries(part).len() == 1)
    }
}

/// The position of the item `id` in `entries`; the rider's own gear (the first) for an id the list lacks.
pub fn position(entries: &[Entry], id: &str) -> usize {
    entries.iter().position(|entry| entry.id == id).unwrap_or(0)
}

fn entry((id, name): (&String, &String)) -> Entry {
    Entry { id: id.clone(), label: name.clone() }
}
