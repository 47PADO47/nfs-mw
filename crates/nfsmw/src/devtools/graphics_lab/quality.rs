//! Shared cost selection; startup shortcuts preserve the saved visual style.
use super::*;
pub(super) use crate::settings::appearance::Quality;
impl Quality {
    pub(super) fn parse(name: &str) -> Option<Self> {
        name.parse().ok()
    }
    pub(super) fn apply(self, settings: &mut Settings) -> RenderExperiments {
        settings.select_overall_quality(self);
        settings.appearance.lighting
    }
}
pub(super) fn startup(mut lab: ResMut<GraphicsLab>, mut settings: ResMut<Settings>) {
    if let Some(options) = lab.startup_options.take() {
        settings.appearance.lighting = options;
    }
    let style = lab.startup_style.take();
    if let Some(style) = style {
        settings.select_visual_style(style);
    }
    if let Some(quality) = lab.startup_quality.take() {
        lab.options = quality.apply(&mut settings);
    }
    if let Some(mode) = lab.startup_local_lights.take() {
        settings.appearance.lighting.local_lamps = mode == "tunnel" || mode == "both";
        settings.appearance.lighting.local_headlights = mode == "headlights" || mode == "both";
    }
    match std::env::var("NFSMW_LAB_NIGHT").as_deref() {
        Ok("1") => {
            settings.appearance.lighting.experimental_night = true;
            // The manual night and the full cycle are exclusive, as in the lab.
            settings.appearance.lighting.full_day_night = false;
            settings.appearance.lighting.local_lamps = true;
            settings.appearance.lighting.local_headlights = true;
            settings.appearance.lighting.local_traffic_headlights = true;
        }
        Ok("0") => settings.appearance.lighting.experimental_night = false,
        _ => {}
    }
    lab.options = settings.appearance.lighting;
}

#[cfg(test)]
mod tests;
