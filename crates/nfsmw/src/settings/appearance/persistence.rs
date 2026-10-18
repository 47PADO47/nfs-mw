//! Versioned stable lighting data. Runtime clock/debug state is never saved.
use super::*;
use serde::{Deserialize, Serialize};

macro_rules! lighting {
    ($($field:ident : $kind:ty),* $(,)?) => {
        #[derive(Serialize, Deserialize)]
        #[serde(default)]
        struct Lighting { projected_shadows: bool, $($field: $kind,)* }
        impl Default for Lighting { fn default() -> Self { Self::from(RenderExperiments::default()) } }
        impl From<RenderExperiments> for Lighting {
            fn from(v: RenderExperiments) -> Self { Self { projected_shadows: v.shadow_projection.is_some(), $($field: v.$field,)* } }
        }
        impl From<Lighting> for RenderExperiments {
            fn from(v: Lighting) -> Self { Self { shadow_projection: v.projected_shadows.then_some(super::shadows::PROJECTION), $($field: v.$field,)* ..Self::default() }.sanitized() }
        }
    }
}
lighting! {
    original_world_materials: bool, sun_shadows: bool, far_shadows: bool, contact_shadows: bool, clearcoat: bool,
    car_skylight: bool, world_ambient: bool, world_ambient_share: f32, world_ambient_tint: f32,
    sky_strength: f32, ground_strength: f32, soft_shadows: bool, dense_shadow_filter: bool, near_shadow_focus: bool, original_shadows: bool,
    wide_shadow_detail: bool, shadow_detail_resolution: u32,
    sun_angle: f32, shadow_resolution: u32, shadow_distance: f32, shadow_filter: f32, contact_length: f32,
}
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Stored {
    display: super::display::DisplayTuning,
    version: u32,
    style: VisualStyle,
    quality: Quality,
    treatment_strength: Option<f32>,
    lighting: Lighting,
}
impl Default for Stored {
    fn default() -> Self {
        Self {
            display: Default::default(),
            version: 1,
            style: VisualStyle::Custom,
            quality: Quality::High,
            treatment_strength: None,
            lighting: Default::default(),
        }
    }
}
pub(super) fn read(value: &toml::Value, origin: &str) -> Option<Appearance> {
    let stored: Stored = match value.clone().try_into() {
        Ok(v) => v,
        Err(error) => {
            log::warn!("{origin}: invalid appearance settings: {error}");
            return None;
        }
    };
    if stored.version != 1 {
        log::warn!("{origin}: unsupported appearance schema {}", stored.version);
        return None;
    }
    Some(Appearance {
        display: stored.display.sanitized(),
        style: stored.style,
        quality: stored.quality,
        treatment_strength: stored.treatment_strength.filter(|v| v.is_finite()).map(|v| v.clamp(0.0, 1.0)),
        lighting: blackbox_gfx::RenderExperiments { original_world_materials: true, ..stored.lighting.into() },
    })
}
pub(super) fn value(value: Appearance) -> anyhow::Result<toml::Value> {
    Ok(toml::Value::try_from(Stored {
        display: value.display.sanitized(),
        version: 1,
        style: value.style,
        quality: value.quality,
        treatment_strength: value.treatment_strength.filter(|v| v.is_finite()).map(|v| v.clamp(0.0, 1.0)),
        lighting: value.lighting.sanitized().into(),
    })?)
}

pub(super) fn merge(existing: Option<&toml::Value>, update: toml::Value) -> anyhow::Result<toml::Value> {
    if let Some(version) = existing.and_then(|v| v.get("version")) {
        anyhow::ensure!(
            version.as_integer() == Some(1),
            "appearance has an unsupported schema; refusing to overwrite it"
        );
    }
    fn combine(old: Option<&toml::Value>, update: toml::Value) -> toml::Value {
        let toml::Value::Table(update) = update else { return update };
        let mut table = old.and_then(toml::Value::as_table).cloned().unwrap_or_default();
        for (key, value) in update {
            let value = combine(table.get(&key), value);
            table.insert(key, value);
        }
        toml::Value::Table(table)
    }
    let clear_strength = update.get("treatment_strength").is_none();
    let mut merged = combine(existing, update);
    if clear_strength && let Some(table) = merged.as_table_mut() {
        table.remove("treatment_strength");
    }
    Ok(merged)
}
