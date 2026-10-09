//! The resolved value of every action for this frame.

use bevy_ecs::resource::Resource;

use super::Action;
use super::bindings::{self, Binding};
use super::snapshot::Snapshot;

/// A button counts as pressed above this value.
const PRESSED: f32 = 0.5;

/// What game code reads. Resolved once per frame from the [`Bindings`] and the device state.
#[derive(Resource, Debug, Clone)]
pub struct ActionState {
    now: [f32; Action::ALL.len()],
    before: [f32; Action::ALL.len()],
    /// The UI had the keyboard when the actions were last resolved, so every game action reads as released.
    typing: bool,
}

impl Default for ActionState {
    fn default() -> Self {
        Self { now: [0.0; Action::ALL.len()], before: [0.0; Action::ALL.len()], typing: false }
    }
}

impl ActionState {
    /// Resolve the actions from the keys held and the analog button pulls, for tests of code that reads actions.
    #[cfg(test)]
    pub fn update_for_test(
        &mut self,
        bindings: &Bindings,
        keys: &[bevy_input::keyboard::KeyCode],
        triggers: &[(bevy_input::gamepad::GamepadButton, f32)],
        ui_focus: bool,
    ) {
        let mut snapshot = Snapshot::default();
        snapshot.keys.extend(keys.iter().copied());
        snapshot.pad_triggers.extend(triggers.iter().copied());
        self.update(bindings, &snapshot, ui_focus);
    }

    /// The action's value: a signed amount for axes, 0 or 1 for buttons.
    pub fn value(&self, action: Action) -> f32 {
        self.now[action.index()]
    }

    /// The UI (the console) has the keyboard: the game actions read as released, which is not the player letting go.
    pub fn typing(&self) -> bool {
        self.typing
    }

    pub fn pressed(&self, action: Action) -> bool {
        self.value(action) > PRESSED
    }

    /// Pressed this frame and not the frame before.
    pub fn just_pressed(&self, action: Action) -> bool {
        self.pressed(action) && self.before[action.index()] <= PRESSED
    }

    /// Resolve every action from `snapshot`, remembering the previous frame for [`Self::just_pressed`].
    /// With `ui_focus` (typing in the console), only the actions that work in the UI stay live.
    pub fn update(&mut self, bindings: &Bindings, snapshot: &Snapshot, ui_focus: bool) {
        self.before = self.now;
        self.typing = ui_focus;
        self.now = [0.0; Action::ALL.len()];
        for b in &bindings.0 {
            self.now[b.action.index()] += b.value(snapshot);
        }
        for a in Action::ALL {
            if a.is_bounded() {
                self.now[a.index()] = self.now[a.index()].clamp(-1.0, 1.0);
            }
            if ui_focus && !a.works_in_ui() {
                self.now[a.index()] = 0.0;
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

impl Bindings {
    /// The defaults plus a wheel's shift paddles, given as the codes of their gamepad buttons.
    pub fn with_paddles(up: Option<u32>, down: Option<u32>) -> Self {
        let mut all = bindings::defaults();
        all.extend(bindings::paddles(up, down));
        Self(all)
    }
}

#[cfg(test)]
mod tests {
    use bevy_input::gamepad::{GamepadAxis, GamepadButton};
    use bevy_input::keyboard::KeyCode;

    use super::*;

    #[test]
    fn keys_and_stick_add_but_movement_stays_bounded() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.keys.insert(KeyCode::KeyW);
        snap.pad_axes.insert(GamepadAxis::LeftStickY, 1.0);
        state.update(&Bindings::default(), &snap, false);
        assert_eq!(state.value(Action::MoveForward), 1.0);

        snap.keys.clear();
        snap.keys.insert(KeyCode::KeyS);
        snap.pad_axes.clear();
        state.update(&Bindings::default(), &snap, false);
        assert_eq!(state.value(Action::MoveForward), -1.0);
    }

    #[test]
    fn just_pressed_fires_once() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.keys.insert(KeyCode::Escape);
        let bindings = Bindings::default();
        state.update(&bindings, &snap, false);
        assert!(state.just_pressed(Action::Cancel));
        state.update(&bindings, &snap, false);
        assert!(state.pressed(Action::Cancel) && !state.just_pressed(Action::Cancel));
    }

    #[test]
    fn scroll_is_not_clamped() {
        let mut state = ActionState::default();
        let snap = Snapshot { scroll: 7.0, ..Snapshot::default() };
        state.update(&Bindings::default(), &snap, false);
        assert_eq!(state.value(Action::Zoom), 7.0);
    }

    #[test]
    fn typing_in_the_ui_silences_the_game_but_not_cancel_or_console() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.keys.extend([KeyCode::KeyW, KeyCode::Escape, KeyCode::F12]);
        state.update(&Bindings::default(), &snap, true);
        assert_eq!(state.value(Action::MoveForward), 0.0);
        assert!(state.pressed(Action::Cancel) && state.pressed(Action::Console));
        state.update(&Bindings::default(), &snap, false);
        assert_eq!(state.value(Action::MoveForward), 1.0);
    }

    #[test]
    fn keyboard_drives() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.keys.extend([KeyCode::KeyW, KeyCode::KeyA, KeyCode::Space, KeyCode::ShiftLeft]);
        state.update(&Bindings::default(), &snap, false);
        assert_eq!(state.value(Action::Throttle), 1.0);
        assert_eq!(state.value(Action::Brake), 0.0);
        assert_eq!(state.value(Action::Steer), -1.0);
        assert!(state.pressed(Action::Handbrake) && state.pressed(Action::Nos));

        // Both steering keys cancel; the arrows work like the letters and do not double up.
        snap.keys.extend([KeyCode::KeyD, KeyCode::ArrowLeft]);
        state.update(&Bindings::default(), &snap, false);
        assert_eq!(state.value(Action::Steer), -1.0);
        snap.keys.insert(KeyCode::ArrowRight);
        state.update(&Bindings::default(), &snap, false);
        assert_eq!(state.value(Action::Steer), 0.0);
    }

    #[test]
    fn q_and_e_shift_and_the_bumpers_too() {
        let bindings = Bindings::default();
        for (key, action) in [(KeyCode::KeyE, Action::ShiftUp), (KeyCode::KeyQ, Action::ShiftDown)] {
            let mut state = ActionState::default();
            let mut snap = Snapshot::default();
            snap.keys.insert(key);
            state.update(&bindings, &snap, false);
            assert!(state.just_pressed(action), "{key:?}");
            let other = if action == Action::ShiftUp { Action::ShiftDown } else { Action::ShiftUp };
            assert!(!state.pressed(other));
        }
        for (button, action) in
            [(GamepadButton::RightTrigger, Action::ShiftUp), (GamepadButton::LeftTrigger, Action::ShiftDown)]
        {
            let mut state = ActionState::default();
            let mut snap = Snapshot::default();
            snap.pad_buttons.insert(button);
            state.update(&bindings, &snap, false);
            assert!(state.just_pressed(action), "{button:?}");
        }
    }

    #[test]
    fn wheel_paddles_shift_once_their_codes_are_given() {
        let mut bindings = Bindings::default();
        let mut snap = Snapshot::default();
        snap.pad_buttons.insert(GamepadButton::Other(12));
        let mut state = ActionState::default();
        state.update(&bindings, &snap, false);
        assert!(!state.pressed(Action::ShiftUp), "an unknown button does nothing by itself");
        bindings.0.extend(bindings::paddles(Some(12), None));
        state.update(&bindings, &snap, false);
        assert!(state.pressed(Action::ShiftUp));
    }

    #[test]
    fn a_left_click_is_the_click_action() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        state.update(&Bindings::default(), &snap, false);
        assert!(!state.pressed(Action::Click));
        snap.buttons.insert(bevy_input::mouse::MouseButton::Left);
        state.update(&Bindings::default(), &snap, false);
        assert!(state.just_pressed(Action::Click));
        assert!(!state.pressed(Action::MenuAccept), "a click does not accept in the menus");
    }

    #[test]
    fn gear_keys_are_single_shots() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.keys.insert(KeyCode::ShiftRight);
        let bindings = Bindings::default();
        state.update(&bindings, &snap, false);
        assert!(state.just_pressed(Action::ShiftUp));
        state.update(&bindings, &snap, false);
        assert!(!state.just_pressed(Action::ShiftUp));
        snap.keys.clear();
        snap.keys.insert(KeyCode::ControlLeft);
        state.update(&bindings, &snap, false);
        assert!(state.just_pressed(Action::ShiftDown));
    }

    #[test]
    fn gamepad_triggers_are_analog_pedals() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.pad_triggers.insert(GamepadButton::RightTrigger2, 0.4);
        snap.pad_triggers.insert(GamepadButton::LeftTrigger2, 1.0);
        snap.pad_axes.insert(GamepadAxis::LeftStickX, -1.0);
        state.update(&Bindings::default(), &snap, false);
        assert!((state.value(Action::Throttle) - 0.4).abs() < 1e-6);
        assert_eq!(state.value(Action::Brake), 1.0);
        assert!((state.value(Action::Steer) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn digital_triggers_still_work() {
        // A pad that reports the trigger only as a pressed button.
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.pad_buttons.insert(GamepadButton::RightTrigger2);
        state.update(&Bindings::default(), &snap, false);
        assert_eq!(state.value(Action::Throttle), 1.0);
    }

    #[test]
    fn a_gentle_stick_push_steers_gently() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.pad_axes.insert(GamepadAxis::LeftStickX, 0.1);
        state.update(&Bindings::default(), &snap, false);
        assert_eq!(state.value(Action::Steer), 0.0, "inside the dead zone");
        snap.pad_axes.insert(GamepadAxis::LeftStickX, 0.6);
        state.update(&Bindings::default(), &snap, false);
        let steer = state.value(Action::Steer);
        assert!(steer > 0.0 && steer < 0.6);
    }

    #[test]
    fn driving_goes_quiet_while_typing() {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.keys.extend([KeyCode::KeyW, KeyCode::KeyR, KeyCode::KeyF]);
        state.update(&Bindings::default(), &snap, true);
        assert!(!state.pressed(Action::Throttle) && !state.pressed(Action::ResetCar));
        assert!(!state.pressed(Action::ToggleCamera));
        state.update(&Bindings::default(), &snap, false);
        assert!(state.pressed(Action::ResetCar) && state.pressed(Action::ToggleCamera));
    }
}
