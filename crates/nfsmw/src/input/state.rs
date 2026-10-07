//! The resolved value of every action for this frame.

use bevy_ecs::resource::Resource;

use super::Action;
use super::bindings::{self, Binding};
use super::snapshot::Snapshot;

/// A button counts as pressed above this value.
const PRESSED: f32 = 0.5;

/// What game code reads. Resolved once per frame from the [`Bindings`] and the device state.
#[derive(Resource, Debug, Default, Clone)]
pub struct ActionState {
    now: [f32; Action::ALL.len()],
    before: [f32; Action::ALL.len()],
}

impl ActionState {
    /// The action's value: a signed amount for axes, 0 or 1 for buttons.
    pub fn value(&self, action: Action) -> f32 {
        self.now[action.index()]
    }

    pub fn pressed(&self, action: Action) -> bool {
        self.value(action) > PRESSED
    }

    /// Pressed this frame and not the frame before.
    pub fn just_pressed(&self, action: Action) -> bool {
        self.pressed(action) && self.before[action.index()] <= PRESSED
    }

    /// Resolve every action from `snapshot`, remembering the previous frame for [`Self::just_pressed`].
    pub fn update(&mut self, bindings: &Bindings, snapshot: &Snapshot) {
        self.before = self.now;
        self.now = [0.0; Action::ALL.len()];
        for b in &bindings.0 {
            self.now[b.action.index()] += b.value(snapshot);
        }
        for a in Action::ALL {
            if a.is_bounded() {
                self.now[a.index()] = self.now[a.index()].clamp(-1.0, 1.0);
            }
        }
    }
}

/// The active key, button and stick assignments.
#[derive(Resource, Debug, Clone)]
pub struct Bindings(pub Vec<Binding>);

impl Default for Bindings {
    fn default() -> Self {
        Self(bindings::defaults())
    }
}

#[cfg(test)]
mod tests {
    use bevy_input::gamepad::GamepadAxis;
    use bevy_input::keyboard::KeyCode;

    use super::*;

    #[test]
    fn keys_and_stick_add_but_movement_stays_bounded() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.keys.insert(KeyCode::KeyW);
        snap.pad_axes.insert(GamepadAxis::LeftStickY, 1.0);
        state.update(&Bindings::default(), &snap);
        assert_eq!(state.value(Action::MoveForward), 1.0);

        snap.keys.clear();
        snap.keys.insert(KeyCode::KeyS);
        snap.pad_axes.clear();
        state.update(&Bindings::default(), &snap);
        assert_eq!(state.value(Action::MoveForward), -1.0);
    }

    #[test]
    fn just_pressed_fires_once() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.keys.insert(KeyCode::Escape);
        let bindings = Bindings::default();
        state.update(&bindings, &snap);
        assert!(state.just_pressed(Action::Cancel));
        state.update(&bindings, &snap);
        assert!(state.pressed(Action::Cancel) && !state.just_pressed(Action::Cancel));
    }

    #[test]
    fn scroll_is_not_clamped() {
        let mut state = ActionState::default();
        let snap = Snapshot { scroll: 7.0, ..Snapshot::default() };
        state.update(&Bindings::default(), &snap);
        assert_eq!(state.value(Action::Zoom), 7.0);
    }
}
