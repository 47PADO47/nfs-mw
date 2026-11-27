//! The free camera under a tools panel (F5 extra options, F8 graphics lab): the panel holds the UI focus, which quiets the game's
//! actions, but the camera keeps the keyboard while egui is not typing and the mouse while the pointer is off the panels, so a view
//! can be framed with the panel open. The pad stays with the panel's navigation.

use bevy_ecs::resource::Resource;
use bevy_input::keyboard::KeyCode;

use super::Action;
use super::bindings::Source;
use super::snapshot::Snapshot;
use super::state::{ActionState, Bindings};

/// The free camera's key actions: move, rise and sink, go faster.
const KEY_ACTIONS: [Action; 4] = [Action::MoveForward, Action::MoveRight, Action::MoveUp, Action::Boost];
/// The free camera's mouse actions: look (the right button held) and the speed on the wheel.
const MOUSE_ACTIONS: [Action; 3] = [Action::LookX, Action::LookY, Action::Zoom];

/// What of the keyboard and the mouse the free camera may read while a tools panel has the UI focus. Set each frame by the
/// panels (`devtools/extra_options.rs`); both off while no such panel is open.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CameraPassthrough {
    pub keys: bool,
    pub mouse: bool,
}

impl CameraPassthrough {
    /// The passthrough for this frame: a tools panel is open with no other panel over it (the console, the garage, the devtools
    /// menu), and egui wanted neither the keyboard (a text field or a focused widget) nor, for the mouse, the pointer.
    pub fn for_panels(tools_open: bool, other_panel: bool, egui_keyboard: bool, egui_pointer: bool) -> Self {
        if !tools_open || other_panel {
            return Self::default();
        }
        Self { keys: !egui_keyboard, mouse: !egui_pointer }
    }

    /// Adds the free camera's keyboard and mouse actions back to `state`, which the UI focus emptied. A held Ctrl is a panel's
    /// shortcut (Ctrl+S saves the lab's preset), not a move.
    pub(super) fn apply(self, state: &mut ActionState, bindings: &Bindings, snapshot: &Snapshot) {
        let ctrl = snapshot.keys.contains(&KeyCode::ControlLeft) || snapshot.keys.contains(&KeyCode::ControlRight);
        let keys = self.keys && !ctrl;
        for binding in &bindings.0 {
            let passed = match binding.source {
                Source::Key(_) => keys && KEY_ACTIONS.contains(&binding.action),
                Source::MouseMotion { .. } | Source::Scroll => self.mouse && MOUSE_ACTIONS.contains(&binding.action),
                _ => false,
            };
            if !passed {
                continue;
            }
            state.add_passed(binding.action, binding.value(snapshot));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_input::mouse::MouseButton;

    use super::*;

    #[test]
    fn only_an_open_tools_panel_alone_passes_input() {
        assert_eq!(CameraPassthrough::for_panels(false, false, false, false), CameraPassthrough::default());
        assert_eq!(CameraPassthrough::for_panels(true, true, false, false), CameraPassthrough::default());
        let both = CameraPassthrough::for_panels(true, false, false, false);
        assert!(both.keys && both.mouse);
        let typing = CameraPassthrough::for_panels(true, false, true, false);
        assert!(!typing.keys && typing.mouse);
        let over_panel = CameraPassthrough::for_panels(true, false, false, true);
        assert!(over_panel.keys && !over_panel.mouse);
    }

    fn resolved(pass: CameraPassthrough, snapshot: &Snapshot) -> ActionState {
        let bindings = Bindings::default();
        let mut state = ActionState::default();
        state.update(&bindings, snapshot, true);
        pass.apply(&mut state, &bindings, snapshot);
        state
    }

    #[test]
    fn the_free_camera_keeps_its_keys_but_not_the_car() {
        let mut snapshot = Snapshot::default();
        snapshot.keys.extend([KeyCode::KeyW, KeyCode::KeyD, KeyCode::ShiftLeft, KeyCode::Space]);
        let state = resolved(CameraPassthrough { keys: true, mouse: false }, &snapshot);
        for action in KEY_ACTIONS {
            assert!(state.pressed(action), "{action:?}");
        }
        for action in [Action::Throttle, Action::Steer, Action::Nos, Action::Handbrake] {
            assert_eq!(state.value(action), 0.0, "{action:?} stays quiet");
        }
        assert!(state.typing(), "the car still reads the UI as having the keyboard");
        let closed = resolved(CameraPassthrough::default(), &snapshot);
        assert_eq!(closed.value(Action::MoveForward), 0.0);
        snapshot.keys.insert(KeyCode::ControlLeft);
        let shortcut = resolved(CameraPassthrough { keys: true, mouse: true }, &snapshot);
        assert_eq!(shortcut.value(Action::MoveForward), 0.0, "Ctrl+S is the lab's save");
    }

    #[test]
    fn the_mouse_looks_with_the_right_button_off_the_panels() {
        let mut snapshot = Snapshot { mouse_delta: (12.0, -4.0), ..Snapshot::default() };
        snapshot.buttons.insert(MouseButton::Right);
        let state = resolved(CameraPassthrough { keys: false, mouse: true }, &snapshot);
        assert!(state.value(Action::LookX) > 0.0 && state.value(Action::LookY) < 0.0);
        assert_eq!(state.value(Action::OrbitX), 0.0);
        let over_panel = resolved(CameraPassthrough { keys: true, mouse: false }, &snapshot);
        assert_eq!(over_panel.value(Action::LookX), 0.0);
    }

    #[test]
    fn the_pad_stays_with_the_panel() {
        let mut snapshot = Snapshot::default();
        snapshot.pad_axes.insert(bevy_input::gamepad::GamepadAxis::LeftStickY, 1.0);
        let state = resolved(CameraPassthrough { keys: true, mouse: true }, &snapshot);
        assert_eq!(state.value(Action::MoveForward), 0.0);
    }
}
