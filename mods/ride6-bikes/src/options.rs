//! The optional parts and upgrades of the bikes: the catalogue the exporter writes (`tools/ride6-exporter/upgrades.py` and
//! `option_export.py`, `docs/formats/ride6-bike-parts.md`) as the importer reads it.
//!
//! `catalogue.json` lists for each bike the aftermarket exhausts (header pipes and silencer), rims, brake discs and bar pads it
//! can take, each with the anchor that puts the part on that bike, and the multipliers its physics asset's upgrade tables give at
//! full level. The parts themselves are `parts.json`, `glb/` and `tex/` of the same folder, shared by every bike.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::partfile::Anchor;

/// What kind of part an asset of the catalogue is, which says how its solids are built and where they sit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The header pipes of one silencer on one chassis.
    Header,
    Muffler,
    /// A front and a rear rim.
    Rim,
    /// A front and a rear brake disc.
    Brake,
    BarPad,
}

impl Kind {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "header" => Some(Self::Header),
            "muffler" => Some(Self::Muffler),
            "rim" => Some(Self::Rim),
            "brake" => Some(Self::Brake),
            "barpad" => Some(Self::BarPad),
            _ => None,
        }
    }
}

/// An aftermarket exhaust on one bike.
#[derive(Debug, Clone, PartialEq)]
pub struct Exhaust {
    /// The row of the `MUFFLER` table (`MUF_Akrapovic_Completo_Standard`): its name.
    pub id: String,
    /// The header and silencer assets (their stems).
    pub header: String,
    pub muffler: String,
    pub header_anchor: Anchor,
    pub muffler_anchor: Anchor,
    /// The anchors of the second, third... silencer of a twin (the catalogue's `muffler_2`, `muffler_3`), in order: the pack
    /// writes one solid per silencer (`options_pack::silencer_name`).
    pub muffler_twins: Vec<Anchor>,
}

/// A pair of rims or of brake discs on one bike.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    /// The `RIMS` row, or the kit level of a brake (`level`).
    pub id: String,
    pub level: u32,
    pub asset: String,
    pub front: Anchor,
    pub back: Anchor,
}

/// A bar pad on one bike.
#[derive(Debug, Clone, PartialEq)]
pub struct Pad {
    pub id: String,
    pub asset: String,
    pub anchor: Anchor,
}

/// What the bike's upgrade tables multiply at full level (1 where a table leaves the number alone).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Upgrade {
    pub peak_torque: f64,
    pub mass: f64,
    pub brake_force: f64,
}

impl Upgrade {
    /// Whether the table changes any number.
    pub fn is_any(&self) -> bool {
        [self.peak_torque, self.mass, self.brake_force].iter().any(|m| (m - 1.0).abs() > 1e-9)
    }
}

/// Everything a bike can be fitted with.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BikeOptions {
    pub exhaust: Vec<Exhaust>,
    pub rims: Vec<Pair>,
    pub brakes: Vec<Pair>,
    pub pads: Vec<Pad>,
    pub upgrade: Option<Upgrade>,
}

impl BikeOptions {
    /// Whether the bike has any part to swap: its stock parts are then solids of their own.
    pub fn has_parts(&self) -> bool {
        !(self.exhaust.is_empty() && self.rims.is_empty() && self.brakes.is_empty() && self.pads.is_empty())
    }
}

/// One data asset of the catalogue: a part shared by the bikes that list it.
#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    pub kind: Kind,
}

/// The catalogue of all bikes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Catalogue {
    pub bikes: BTreeMap<String, BikeOptions>,
    pub assets: BTreeMap<String, Asset>,
}

fn anchor(value: &Value) -> Option<Anchor> {
    let list = |key: &str, n: usize| -> Option<Vec<f32>> {
        let items = value.get(key)?.as_array()?;
        let numbers: Vec<f32> = items.iter().filter_map(|v| v.as_f64().map(|x| x as f32)).collect();
        (numbers.len() == n).then_some(numbers)
    };
    let (t, r) = (list("translation", 3)?, list("rotation", 4)?);
    // The scale is optional: a rigid anchor has none. A list of three numbers is one per axis, a single number is all three.
    let scale = match (list("scale", 3), value.get("scale").and_then(Value::as_f64)) {
        (Some(s), _) => [s[0], s[1], s[2]],
        (None, Some(one)) => [one as f32; 3],
        (None, None) => [1.0; 3],
    };
    Some(Anchor {
        bone: value.get("bone").and_then(Value::as_str).map(str::to_owned),
        translation: [t[0], t[1], t[2]],
        rotation: [r[0], r[1], r[2], r[3]],
        scale,
    })
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn exhaust(value: &Value) -> Option<Exhaust> {
    let anchors = value.get("anchors")?;
    Some(Exhaust {
        id: text(value, "id")?,
        header: text(value, "header")?,
        muffler: text(value, "muffler")?,
        header_anchor: anchor(anchors.get("header")?)?,
        muffler_anchor: anchor(anchors.get("muffler")?)?,
        muffler_twins: twins(anchors),
    })
}

/// The anchors `muffler_2`, `muffler_3`... of a twin silencer, in order; the run stops at the first number the catalogue lacks.
fn twins(anchors: &Value) -> Vec<Anchor> {
    (2..).map_while(|n| anchor(anchors.get(format!("muffler_{n}"))?)).collect()
}

fn pair(value: &Value) -> Option<Pair> {
    let anchors = value.get("anchors")?;
    let level = value.get("level").and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok()).unwrap_or(0);
    Some(Pair {
        id: text(value, "id").unwrap_or_default(),
        level,
        asset: text(value, "asset")?,
        front: anchor(anchors.get("front")?)?,
        back: anchor(anchors.get("back")?)?,
    })
}

fn pad(value: &Value) -> Option<Pad> {
    Some(Pad {
        id: text(value, "id")?,
        asset: text(value, "asset")?,
        anchor: anchor(value.get("anchors")?.get("body")?)?,
    })
}

fn list<T>(value: &Value, key: &str, read: fn(&Value) -> Option<T>) -> Vec<T> {
    let items = value.get(key).and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    items.iter().filter_map(read).collect()
}

fn upgrade(value: &Value) -> Option<Upgrade> {
    let full = value.get("upgrade")?.get("full")?;
    let multiplier = |key: &str| full.get(key).and_then(Value::as_f64).unwrap_or(1.0);
    Some(Upgrade {
        peak_torque: multiplier("peak_torque"),
        mass: multiplier("mass"),
        brake_force: multiplier("brake_force"),
    })
}

impl Catalogue {
    /// Read `catalogue.json` of `dir`.
    pub fn read(dir: &Path) -> Result<Self> {
        let path = dir.join("catalogue.json");
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("reading {}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self> {
        let doc: Value = serde_json::from_str(text).context("catalogue.json is not JSON")?;
        let schema = doc.get("schema").and_then(Value::as_u64).unwrap_or(0);
        anyhow::ensure!(schema == 1, "catalogue.json schema {schema} is not supported (this importer reads schema 1)");
        let mut out = Self::default();
        for (stem, asset) in doc.get("assets").and_then(Value::as_object).into_iter().flatten() {
            let Some(kind) = asset.get("kind").and_then(Value::as_str).and_then(Kind::parse) else { continue };
            out.assets.insert(stem.clone(), Asset { kind });
        }
        for (vehicle, bike) in doc.get("bikes").and_then(Value::as_object).into_iter().flatten() {
            let options = BikeOptions {
                exhaust: list(bike, "exhaust", exhaust),
                rims: list(bike, "rims", pair),
                brakes: list(bike, "brakes", pair),
                pads: list(bike, "barpad", pad),
                upgrade: upgrade(bike),
            };
            out.bikes.insert(vehicle.clone(), options);
        }
        Ok(out)
    }

    /// The options of the bike with the vehicle id `vehicle`; none for a bike the catalogue does not list.
    pub fn bike(&self, vehicle: &str) -> Option<&BikeOptions> {
        self.bikes.get(vehicle)
    }

    /// The stems of every asset some bike of `vehicles` uses, in name order.
    pub fn used_by<'a>(&self, vehicles: impl Iterator<Item = &'a str>) -> Vec<String> {
        let mut stems: Vec<String> = Vec::new();
        for options in vehicles.filter_map(|vehicle| self.bike(vehicle)) {
            for item in &options.exhaust {
                stems.extend([item.header.clone(), item.muffler.clone()]);
            }
            stems.extend(options.rims.iter().chain(&options.brakes).map(|p| p.asset.clone()));
            stems.extend(options.pads.iter().map(|p| p.asset.clone()));
        }
        stems.sort();
        stems.dedup();
        stems
    }
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;
