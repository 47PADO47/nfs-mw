//! The per-car override file `cars/<car>.toml` (parameter-mapping.md section 5.4).

use serde::Deserialize;

use super::{CarClass, Compound};
use crate::keys::{BIAS_RANGE, KeyTable, check};
use crate::presets::parse::SCHEMA;

/// The shape of a car file before its checks. The `[tyre]` table is read as a plain table so its `front` and `rear` children
/// can be split off and the rest still rejects unknown keys.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    schema: u32,
    compound: Option<String>,
    class: Option<String>,
    axle_angle_bias_deg: Option<f64>,
    tyre: Option<toml::Table>,
}

/// A parsed and checked car file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CarFile {
    /// Forces the compound instead of the grip level.
    pub compound: Option<Compound>,
    /// Forces the car class.
    pub class: Option<CarClass>,
    /// Replaces the axle bias of the layout.
    pub bias: Option<f64>,
    /// `[tyre]`: replaces the value on both axles.
    pub both: KeyTable,
    /// `[tyre.front]`: replaces the value on the front axle only.
    pub front: KeyTable,
    /// `[tyre.rear]`: replaces the value on the rear axle only.
    pub rear: KeyTable,
}

/// Parse and check the text of a car file. The error is one line naming the key and the reason.
pub fn parse_car_file(text: &str) -> Result<CarFile, String> {
    let raw: Raw = toml::from_str(text).map_err(|error| error.message().to_owned())?;
    if raw.schema != SCHEMA {
        return Err(format!("schema = {} is not supported (expected {SCHEMA})", raw.schema));
    }
    let compound = match raw.compound.as_deref() {
        None => None,
        Some(name) => Some(Compound::parse(name).ok_or_else(|| format!("\"{name}\" is not a compound name"))?),
    };
    let class = match raw.class.as_deref() {
        None => None,
        Some(name) => Some(CarClass::parse(name).ok_or_else(|| format!("\"{name}\" is not a class name"))?),
    };
    if let Some(bias) = raw.axle_angle_bias_deg {
        check("axle_angle_bias_deg", bias, Some(BIAS_RANGE))?;
    }
    let mut file = CarFile { compound, class, bias: raw.axle_angle_bias_deg, ..CarFile::default() };
    let Some(mut tyre) = raw.tyre else {
        return Ok(file);
    };
    let front = tyre.remove("front");
    let rear = tyre.remove("rear");
    file.both = table_of(toml::Value::Table(tyre), "[tyre]")?;
    file.front = optional_table(front, "[tyre.front]")?;
    file.rear = optional_table(rear, "[tyre.rear]")?;
    Ok(file)
}

fn optional_table(value: Option<toml::Value>, place: &str) -> Result<KeyTable, String> {
    let Some(value) = value else {
        return Ok(KeyTable::default());
    };
    table_of(value, place)
}

/// One `[tyre]` table: known keys, finite numbers inside their ranges, no axle bias (it is a top-level key here).
fn table_of(value: toml::Value, place: &str) -> Result<KeyTable, String> {
    let table: KeyTable = value.try_into().map_err(|error: toml::de::Error| format!("{place}: {}", error.message()))?;
    table.validate().map_err(|reason| format!("{place}: {reason}"))?;
    if table.axle_angle_bias_deg.is_some() {
        return Err(format!("{place}: set axle_angle_bias_deg at the top level of the file"));
    }
    Ok(table)
}
