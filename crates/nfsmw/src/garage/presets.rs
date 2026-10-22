//! Persistent car setups submitted through the normal validated garage transaction.
use super::{Model, Selection};
use anyhow::{Context, Result, ensure};
use nfsmw_data::car::physics::{PerformanceCategory, PerformancePackage, tuning::TuningSelection};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Part {
    slot: usize,
    hash: u32,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    version: u32,
    car: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<[u8; 3]>,
    levels: [u8; 7],
    junkman: u8,
    tuning: [[i8; 7]; 3],
    active_tuning: u8,
    parts: Vec<Part>,
    #[serde(default)]
    rim_color: Option<[u8; 3]>,
    #[serde(default)]
    vinyl_color1: Option<[u8; 3]>,
    #[serde(default)]
    vinyl_color2: Option<[u8; 3]>,
    #[serde(default)]
    vinyl_color3: Option<[u8; 3]>,
    #[serde(default)]
    gauges: super::GaugeSelection,
}

fn encode(selection: &Selection) -> Result<String> {
    selection.tuning.validate()?;
    let stored = Stored {
        version: 1,
        rim_color: selection.parts.rim_color,
        vinyl_color1: selection.parts.vinyl_colors[0],
        vinyl_color2: selection.parts.vinyl_colors[1],
        vinyl_color3: selection.parts.vinyl_colors[2],
        gauges: selection.gauges.clone(),
        car: selection.car.clone(),
        color: selection.color,
        levels: selection.performance.levels,
        junkman: selection.performance.junkman,
        tuning: selection.tuning.profiles,
        active_tuning: selection.tuning.active,
        parts: selection
            .parts
            .parts
            .iter()
            .enumerate()
            .filter_map(|(slot, hash)| hash.map(|hash| Part { slot, hash }))
            .collect(),
    };
    toml::to_string(&stored).context("Serializing car preset")
}

pub(crate) fn decode(text: &str, model: &Model) -> Result<Selection> {
    let stored: Stored = toml::from_str(text).context("Invalid car preset")?;
    ensure!(stored.version == 1, "Unsupported car preset schema");
    let car = model.cars.iter().find(|c| c.folder == stored.car).context("Preset car is not installed")?;
    ensure!(car.unavailable.is_none(), "Preset car is unavailable");
    ensure!(stored.color.is_none() || car.paintable, "Preset paint requires a paintable car");
    let performance = PerformancePackage { levels: stored.levels, junkman: stored.junkman };
    ensure!(performance.junkman & 128 == 0, "Invalid Junkman category");
    for category in PerformanceCategory::ALL {
        ensure!(
            performance.get(category) <= car.performance.categories[category as usize].max_level,
            "Preset {} stage is unavailable",
            category.label()
        );
        ensure!(
            !performance.has_junkman(category) || car.performance.junkman_available(category, performance),
            "Preset {} Junkman package is unavailable",
            category.label()
        );
    }
    let tuning = TuningSelection { profiles: stored.tuning, active: stored.active_tuning };
    tuning.validate()?;
    let mut parts = nfsmw_data::car::customization::Selection::default();
    for part in stored.parts {
        let slot = parts.parts.get_mut(part.slot).context("Invalid preset part slot")?;
        ensure!(slot.is_none(), "Duplicate preset part slot");
        *slot = Some(part.hash);
    }
    parts.rim_color = stored.rim_color;
    parts.vinyl_colors = [stored.vinyl_color1, stored.vinyl_color2, stored.vinyl_color3];
    if let Some(catalog) = &car.parts {
        stored.gauges.validate(catalog)?;
        catalog.validate(&parts)?;
    }
    Ok(Selection { car: stored.car, color: stored.color, performance, tuning, parts, gauges: stored.gauges })
}

pub(super) fn show(ui: &mut egui::Ui, model: &mut Model) {
    match model.preset_picker.show(ui, "garage-presets.toml") {
        Some(crate::preset_store::Action::Save) => {
            let payload = encode(&model.draft).and_then(|text| {
                decode(&text, model)?;
                Ok(text)
            });
            model.preset_picker.save(payload);
        }
        Some(crate::preset_store::Action::Load) => {
            let result = model.preset_picker.load().and_then(|text| decode(&text, model));
            match result {
                Ok(selection) => {
                    model.category =
                        model.cars.iter().find(|c| c.folder == selection.car).expect("validated car").category;
                    model.draft = selection;
                    model.parts_filter = Default::default();
                    model.message = None;
                    model.preset_picker.report(Ok("Car preset loaded.".into()));
                }
                Err(error) => model.preset_picker.report(Err(error)),
            }
        }
        None => {}
    }
}

#[cfg(test)]
mod tests;
