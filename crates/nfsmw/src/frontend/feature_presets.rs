//! Coarse feature presets share the same budgets as the overall quality control.
use super::graphics_options::GraphicsSetting;
use super::ids::LABEL_OFF;
use super::options::{Data, message};
use crate::settings::{GraphicsQuality as Quality, Partial, Settings};

pub(super) fn data(row: GraphicsSetting, settings: &Settings) -> Option<Data> {
    let (quality, off) = match row {
        GraphicsSetting::ShadowPreset => {
            (settings.shadow_budget(), settings.shadows == crate::settings::ShadowQuality::Off)
        }
        GraphicsSetting::ReflectionPreset => {
            (settings.reflection_budget(), settings.reflections == blackbox_gfx::ReflectionQuality::Off)
        }
        _ => return None,
    };
    Some(match (off, quality) {
        (true, _) => Data::Label(LABEL_OFF),
        (false, Some(quality)) => Data::Text(quality.label()),
        _ => message("options-custom"),
    })
}

pub(super) fn step(row: GraphicsSetting, settings: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
    let current = match row {
        GraphicsSetting::ShadowPreset => settings.shadow_budget(),
        GraphicsSetting::ReflectionPreset => settings.reflection_budget(),
        _ => return false,
    };
    let mut all = vec![None];
    all.extend(Quality::offered(settings.renderer).iter().copied().map(Some));
    let next = super::post_options::pick(&all, current, forward);
    match row {
        GraphicsSetting::ShadowPreset => settings.select_shadow_budget(next),
        GraphicsSetting::ReflectionPreset => settings.select_reflection_budget(next),
        _ => unreachable!(),
    }
    *changed = settings.appearance_layer().or(*changed);
    true
}
