//! The names the controls list gives the HUD's actions, from the Fluent catalogue (`i18n/en/keys.ftl`). The FEng
//! controls packages have eight fixed rows, none of them for these actions (`frontend/controls_screen.rs`); the Custom
//! controls list names its rows for them with these. The `keys` listing (`describe.rs`) keeps its own English labels.

// The Custom controls list that reads these is built in a later PR of the Custom HUD stack.
#![allow(dead_code)]

use super::Action;

/// The controls list's name for one of the HUD's actions, or `None` for any other action.
pub fn label(action: Action) -> Option<String> {
    let id = match action {
        Action::MapMode => "keys-action-map-mode",
        Action::MapExpand => "keys-action-map-expand",
        Action::MapZoomIn => "keys-action-map-zoom-in",
        Action::MapZoomOut => "keys-action-map-zoom-out",
        Action::MapOrientation => "keys-action-map-orientation",
        Action::ToggleHud => "keys-action-toggle-hud",
        _ => return None,
    };
    Some(crate::i18n::text(id, &crate::i18n::Args::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_hud_action_has_a_name_and_the_others_none() {
        assert_eq!(label(Action::MapMode).as_deref(), Some("Map mode"));
        assert_eq!(label(Action::ToggleHud).as_deref(), Some("Show or hide the HUD"));
        let named = [
            Action::MapMode,
            Action::MapExpand,
            Action::MapZoomIn,
            Action::MapZoomOut,
            Action::MapOrientation,
            Action::ToggleHud,
        ];
        for action in named {
            let text = label(action).unwrap();
            assert!(!text.starts_with("keys-action-"), "{action:?} resolves in the catalogue, got {text}");
        }
        assert_eq!(label(Action::Throttle), None);
    }
}
