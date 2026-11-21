//! The parser and validator of a preset file (parameter-mapping.md section 5.4). A file is rejected whole, with one reason.

use serde::Deserialize;

use super::{CarClass, ClassPreset, Compound, CompoundPreset, UnitsTable};
use crate::keys::{KeyTable, check};

/// The schema version of every file.
pub const SCHEMA: u32 = 1;

/// The shape of a preset file before its checks.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    schema: u32,
    kind: String,
    name: String,
    level_ref: Option<f64>,
    grip_slope: Option<f64>,
    tyre: Option<KeyTable>,
    mul: Option<KeyTable>,
    add: Option<KeyTable>,
    units: Option<UnitsTable>,
}

/// A parsed preset file.
#[derive(Debug, Clone, PartialEq)]
pub enum Preset {
    /// `kind = "compound"`.
    Compound(Box<CompoundPreset>),
    /// `kind = "class"`.
    Class(Box<ClassPreset>),
}

/// Parse and check the text of the preset file called `file` (`compound-sport.toml`, `class-super.toml`).
pub fn parse_preset(file: &str, text: &str) -> Result<Preset, String> {
    let raw: Raw = toml::from_str(text).map_err(|error| error.message().to_owned())?;
    if raw.schema != SCHEMA {
        return Err(format!("schema = {} is not supported (expected {SCHEMA})", raw.schema));
    }
    let stem = file.strip_suffix(".toml").unwrap_or(file);
    let expected = format!("{}-{}", raw.kind, raw.name);
    if stem != expected {
        return Err(format!("kind and name say \"{expected}\", but the file is called \"{file}\""));
    }
    match raw.kind.as_str() {
        "compound" => compound(raw),
        "class" => class(raw),
        other => Err(format!("kind \"{other}\" is not \"compound\" or \"class\"")),
    }
}

fn compound(raw: Raw) -> Result<Preset, String> {
    let compound = Compound::parse(&raw.name).ok_or_else(|| format!("\"{}\" is not a compound name", raw.name))?;
    if raw.mul.is_some() || raw.add.is_some() {
        return Err("a compound preset has no [mul] or [add] table".to_owned());
    }
    let level_ref = raw.level_ref.ok_or("a compound preset needs level_ref")?;
    let grip_slope = raw.grip_slope.ok_or("a compound preset needs grip_slope")?;
    check("level_ref", level_ref, Some((0.2, 3.0)))?;
    check("grip_slope", grip_slope, Some((0.0, 2.0)))?;
    let tyre = raw.tyre.unwrap_or_default();
    tyre.validate()?;
    let units = raw.units.unwrap_or_default();
    units.validate()?;
    Ok(Preset::Compound(Box::new(CompoundPreset { compound, level_ref, grip_slope, tyre, units })))
}

fn class(raw: Raw) -> Result<Preset, String> {
    let class = CarClass::parse(&raw.name).ok_or_else(|| format!("\"{}\" is not a class name", raw.name))?;
    if raw.tyre.is_some() {
        return Err("a class preset has no [tyre] table (use [mul] and [add])".to_owned());
    }
    if raw.units.is_some() {
        return Err("a class preset has no [units] table".to_owned());
    }
    if raw.level_ref.is_some() || raw.grip_slope.is_some() {
        return Err("level_ref and grip_slope belong to a compound preset".to_owned());
    }
    let mul = raw.mul.unwrap_or_default();
    let add = raw.add.unwrap_or_default();
    factors(&mul, "mul", (0.1, 10.0))?;
    factors(&add, "add", (-50.0, 50.0))?;
    Ok(Preset::Class(Box::new(ClassPreset { class, mul, add })))
}

/// A `[mul]` or `[add]` table: known keys (the table type guarantees it), finite numbers inside `range`, no axle bias.
fn factors(table: &KeyTable, which: &str, range: (f64, f64)) -> Result<(), String> {
    if table.axle_angle_bias_deg.is_some() {
        return Err(format!("[{which}] cannot change axle_angle_bias_deg"));
    }
    for (key, value) in table.entries() {
        check(&format!("{which}.{key}"), value, Some(range))?;
    }
    Ok(())
}
