//! The exporter's gear catalogue (`catalogue.json`, written by `tools/ride6-exporter/gear.py`): the items of the rider editor
//! with the keys of their meshes and pictures. Only what the importer reads is declared; the format is in
//! `docs/formats/ride6-rider-gear.md`.

use std::collections::HashMap;

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

/// A file of the catalogue, by its path inside the catalogue folder.
#[derive(Debug, Clone, Deserialize)]
pub struct File {
    pub file: String,
}

/// A body: the suit's shape and the sex of the rider.
#[derive(Debug, Clone, Deserialize)]
pub struct Body {
    pub id: String,
    pub gender: String,
    pub shape: u32,
    pub mesh: String,
}

/// The visor of a helmet: a mesh of its own and its picture.
#[derive(Debug, Clone, Deserialize)]
pub struct Visor {
    pub mesh: String,
    pub color: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Helmet {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub brand: String,
    pub mesh: String,
    pub color: String,
    pub visor: Option<Visor>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Suit {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub brand: String,
    pub color: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Gloves {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub brand: String,
    pub rgb: [u8; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct Boots {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub brand: String,
    pub mesh: String,
    pub color: String,
}

/// The whole catalogue.
#[derive(Debug, Clone, Deserialize)]
pub struct Catalogue {
    pub version: u32,
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
    /// Mesh key to file.
    #[serde(default)]
    pub meshes: HashMap<String, File>,
    /// Picture key to file.
    #[serde(default)]
    pub textures: HashMap<String, File>,
}

/// Reads a `catalogue.json`.
pub fn parse(bytes: &[u8]) -> Result<Catalogue> {
    let catalogue: Catalogue = serde_json::from_slice(bytes).context("catalogue.json")?;
    ensure!(catalogue.version == 1, "catalogue.json has version {}, this importer reads 1", catalogue.version);
    Ok(catalogue)
}
