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
    lab.options = settings.appearance.lighting;
}

#[cfg(test)]
mod tests;
