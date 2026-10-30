//! Which clip set a bike's rider uses, and the `styles.json` file that tells the game which graph each set has
//! (`docs/specs/ride6-rider-styles.md`).
//!
//! RIDE 6 picks the rider's clips by the integer `BikeType` of its animation blueprint (`graph_sets`). The value is not in the
//! bike's rider data; it follows from two things the bike's data does say:
//!
//! 1. the row of the bike in `RiderAnimationCorrection_DT` has an `EBikeTypeOverride` (`RACING`, `STREET`, `MOTARD`, `SCOOTER`,
//!    `ENDURO`, `BAGGER`, or `NOOVERRIDE`): 26 of the 364 rows override, and
//! 2. every bike's physics asset runs one input handler class (`Default`, `2502T`, `Naked`, `Motard`, `Maxienduro`, `Scooter`,
//!    `ScooterGear`, `Baggers`), which the game's camera containers follow exactly (316 of 316 bikes agree).
//!
//! The override wins when there is one. That the engine's `BikeType` follows the input handler when there is no override is
//! an inference (the native code that sets the blueprint's variable was not read); it is a named choice of the spec.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use super::graph_reverse::{RATE, RATE_SPEED_KMH, START_SECONDS, START_SPEED_KMH};
use super::graph_sets::{self, ClipSet};

/// The file name in `files/RIDER/`.
pub const FILE: &str = "styles.json";
/// The version the game checks.
pub const VERSION: u32 = 1;

/// Where a bike's type came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The row of the animation correction table names a type.
    Override,
    /// The bike's input handler class.
    Handler,
}

impl Source {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Override => "correction-table",
            Self::Handler => "input-handler",
        }
    }
}

/// The type of an input handler class by its name (`Motard_InputHandler`, with or without the `_C` of the class).
pub fn from_handler(handler: &str) -> Option<&'static ClipSet> {
    let name = handler.trim_end_matches("_C").trim_end_matches("_InputHandler").to_ascii_lowercase();
    match name.as_str() {
        "default" | "2502t" => Some(&graph_sets::RACE),
        "naked" | "baggers" => Some(&graph_sets::NAKED),
        "motard" => Some(&graph_sets::MOTARD),
        "maxienduro" => Some(&graph_sets::ENDURO),
        "scooter" | "scootergear" => Some(&graph_sets::SCOOTER),
        _ => None,
    }
}

/// The type an `EBikeTypeOverride` names (`RACING`, or `EBikeTypeOverride::RACING`); `None` for `NOOVERRIDE` and for a name
/// that is not known.
pub fn from_override(name: &str) -> Option<&'static ClipSet> {
    match name.rsplit("::").next().unwrap_or(name) {
        "RACING" => Some(&graph_sets::RACE),
        "STREET" | "BAGGER" => Some(&graph_sets::NAKED),
        "MOTARD" => Some(&graph_sets::MOTARD),
        "SCOOTER" => Some(&graph_sets::SCOOTER),
        "ENDURO" => Some(&graph_sets::ENDURO),
        _ => None,
    }
}

/// The type of a bike: the override of its correction row, else its input handler's. `None` when neither says.
pub fn choose(override_name: Option<&str>, handler: Option<&str>) -> Option<(&'static ClipSet, Source)> {
    if let Some(set) = override_name.and_then(from_override) {
        return Some((set, Source::Override));
    }
    handler.and_then(from_handler).map(|set| (set, Source::Handler))
}

/// The name of the input handler class a vehicle's physics file runs: the last path part of the `VehicleElements` entry whose
/// class path has `InputHandler`.
pub fn input_handler(vehicle: &Value) -> Option<String> {
    let elements = vehicle.get("Properties")?.get("VehicleElements")?.as_array()?;
    for element in elements {
        let path = element.get("ObjectPath").and_then(Value::as_str)?;
        if !path.contains("InputHandler") {
            continue;
        }
        let class = path.rsplit('/').next()?.split('.').next()?;
        return Some(class.to_owned());
    }
    None
}

/// The input handler of the vehicle whose resolved physics file is at `path`.
pub fn input_handler_of(path: &Path) -> Result<Option<String>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let vehicle: Value = serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))?;
    Ok(input_handler(&vehicle))
}

/// The graph file of a set, in `files/RIDER/`.
pub fn graph_file(set: &ClipSet) -> String {
    format!("graph_{}.json", set.name)
}

/// One entry of `styles.json`: the graph, the riding style it was built for and the numbers of the reverse walk.
pub fn entry(set: &ClipSet, riding_style: u8, loop_seconds: Option<f32>) -> Value {
    let mut out = json!({
        "bike_type": set.bike_type,
        "graph": graph_file(set),
        "riding_style": riding_style,
    });
    if let Some(seconds) = loop_seconds {
        out["reverse"] = json!({
            "start_seconds": [START_SECONDS.0, START_SECONDS.1],
            "start_speed_kmh": START_SPEED_KMH,
            "rate_speed_kmh": [RATE_SPEED_KMH.0, RATE_SPEED_KMH.1],
            "rate": [RATE.0, RATE.1],
            "loop_seconds": seconds,
        });
    }
    out
}

/// The `styles.json` text from its entries, `(set, entry)`.
pub fn table(entries: &[(&ClipSet, Value)]) -> Value {
    let styles: serde_json::Map<String, Value> =
        entries.iter().map(|(set, entry)| (set.name.to_owned(), entry.clone())).collect();
    json!({ "version": VERSION, "styles": styles })
}

#[cfg(test)]
#[path = "style_tests.rs"]
mod tests;
