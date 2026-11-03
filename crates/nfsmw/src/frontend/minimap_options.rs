//! Context rows share the existing minimap choices and effects.
use super::*;
use crate::settings::MinimapContext;

pub(super) fn next(mode: MinimapMode, forward: bool) -> MinimapMode {
    let modes = [MinimapMode::Fixed, MinimapMode::Rotating, MinimapMode::Off];
    let at = modes.iter().position(|candidate| *candidate == mode).unwrap_or(0);
    modes[cycle(at, modes.len(), forward)]
}

pub(super) fn data(mode: MinimapMode) -> Data {
    match mode {
        MinimapMode::Fixed => message("options-minimap-fixed"),
        MinimapMode::Rotating => message("options-minimap-rotating"),
        MinimapMode::Off => Data::Label(LABEL_OFF),
    }
}

pub(super) fn step(context: MinimapContext, settings: &mut Settings, changes: &mut Partial, forward: bool) {
    let mode = Some(next(settings.minimap_mode(context), forward));
    match context {
        MinimapContext::FreeRoam => {
            settings.free_roam_map_mode = mode;
            changes.free_roam_map_mode = mode;
        }
        MinimapContext::Race => {
            settings.race_map_mode = mode;
            changes.race_map_mode = mode;
        }
    }
}
