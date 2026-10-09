//! What the driver asks of the car this step, from the player or from a script.

use blackbox_vehicle::drivetrain::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE};

use super::Drive;
use crate::input::{Action, ActionState};
use crate::settings::{Transmission, WheelOptions};

impl Drive {
    /// Who changes gear from now on (the transmission setting).
    pub fn set_transmission(&mut self, transmission: Transmission) {
        self.transmission = transmission;
    }

    /// Whether the clutch pedal and the H-shifter are in use from now on.
    pub fn set_wheel_options(&mut self, wheel: WheelOptions) {
        self.wheel = wheel;
    }
}

/// Pedals, steering and buttons for one physics step.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct DriveInput {
    /// 0..1.
    pub throttle: f32,
    /// 0..1.
    pub brake: f32,
    /// -1 (left) ..1 (right).
    pub steer: f32,
    pub handbrake: bool,
    pub nos: bool,
    /// Shift requests; true for the one step after the button went down.
    pub shift_up: bool,
    pub shift_down: bool,
    /// A gear asked for by number (reverse 0, neutral 1, first 2, ...), for the one step after a gear key went down; or,
    /// with an H-shifter, the gear the selector holds every step.
    pub gear_select: Option<usize>,
    /// Clutch pedal, 0 (released) to 1; read only when the car has the manual clutch switched on.
    pub clutch: f32,
    /// The reset button went down: put the car back on the nearest road.
    pub reset: bool,
    /// A scripted sputter pop for the exhaust flames (`pop` in a `--drive-script`; no key does it).
    pub pop: bool,
}

/// The actions that ask for a gear by number, with the gear id each one means.
const GEAR_ACTIONS: [(Action, usize); 9] = [
    (Action::GearReverse, GEAR_REVERSE),
    (Action::GearNeutral, GEAR_NEUTRAL),
    (Action::Gear1, GEAR_FIRST),
    (Action::Gear2, GEAR_FIRST + 1),
    (Action::Gear3, GEAR_FIRST + 2),
    (Action::Gear4, GEAR_FIRST + 3),
    (Action::Gear5, GEAR_FIRST + 4),
    (Action::Gear6, GEAR_FIRST + 5),
    (Action::Gear7, GEAR_FIRST + 6),
];

/// The gear the gear actions ask for. Gear keys ask once, when they go down. A selector that holds a gear (an
/// H-pattern shifter) asks all the time: the gear whose button is held, or neutral when none is, except while the
/// UI has the keyboard and every button reads as released.
fn selected_gear(actions: &ActionState, h_shifter: bool) -> Option<usize> {
    if !h_shifter {
        return GEAR_ACTIONS.iter().find(|(action, _)| actions.just_pressed(*action)).map(|(_, gear)| *gear);
    }
    if actions.typing() {
        return None;
    }
    let held = GEAR_ACTIONS.iter().find(|(action, _)| actions.pressed(*action));
    Some(held.map_or(GEAR_NEUTRAL, |(_, gear)| *gear))
}

impl DriveInput {
    pub fn from_actions(actions: &ActionState, wheel: WheelOptions) -> Self {
        Self {
            throttle: actions.value(Action::Throttle).clamp(0.0, 1.0),
            brake: actions.value(Action::Brake).clamp(0.0, 1.0),
            steer: actions.value(Action::Steer).clamp(-1.0, 1.0),
            handbrake: actions.pressed(Action::Handbrake),
            nos: actions.pressed(Action::Nos),
            shift_up: actions.just_pressed(Action::ShiftUp),
            shift_down: actions.just_pressed(Action::ShiftDown),
            gear_select: selected_gear(actions, wheel.h_shifter),
            clutch: actions.value(Action::Clutch).clamp(0.0, 1.0),
            reset: actions.just_pressed(Action::ResetCar),
            pop: false,
        }
    }

    /// The same input with the one-shot shift requests cleared, for the second and later physics
    /// steps of a frame.
    pub fn held(self) -> Self {
        Self { shift_up: false, shift_down: false, gear_select: None, reset: false, pop: false, ..self }
    }
}

/// Button presses that wait for a physics step. The physics runs at a fixed 60 Hz, so a frame at a higher rate
/// sometimes runs no step at all; a press that came in such a frame must not be lost (the original queues its
/// shift actions the same way).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Presses {
    shift_up: bool,
    shift_down: bool,
    gear_select: Option<usize>,
    reset: bool,
}

impl Presses {
    /// Remembers the presses of this frame's input.
    pub fn note(&mut self, input: &DriveInput) {
        self.shift_up |= input.shift_up;
        self.shift_down |= input.shift_down;
        self.gear_select = input.gear_select.or(self.gear_select);
        self.reset |= input.reset;
    }

    /// `input` with the waiting presses in place of its own, which are now consumed.
    pub fn take_into(&mut self, input: DriveInput) -> DriveInput {
        let waiting = std::mem::take(self);
        DriveInput {
            shift_up: waiting.shift_up,
            shift_down: waiting.shift_down,
            gear_select: waiting.gear_select,
            reset: waiting.reset,
            ..input
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_input::keyboard::KeyCode;

    use super::*;
    use crate::input::Bindings;

    #[test]
    fn a_press_waits_for_the_next_step() {
        let mut presses = Presses::default();
        let pressed = DriveInput { shift_up: true, reset: true, ..DriveInput::default() };
        // A frame with no physics step: the press is noted and nothing consumes it.
        presses.note(&pressed);
        // The next frame has no press of its own but a step.
        let later = DriveInput { throttle: 0.5, ..DriveInput::default() };
        presses.note(&later);
        let step = presses.take_into(later);
        assert!(step.shift_up && step.reset && !step.shift_down);
        assert_eq!(step.throttle, 0.5);
        assert!(!presses.take_into(later).shift_up, "a press is taken once");
    }

    #[test]
    fn a_gear_key_press_waits_for_the_next_step_too() {
        let mut presses = Presses::default();
        presses.note(&DriveInput { gear_select: Some(GEAR_FIRST + 2), ..DriveInput::default() });
        let later = DriveInput::default();
        presses.note(&later);
        assert_eq!(presses.take_into(later).gear_select, Some(GEAR_FIRST + 2));
        assert_eq!(presses.take_into(later).gear_select, None, "taken once");
        assert_eq!(DriveInput { gear_select: Some(3), ..DriveInput::default() }.held().gear_select, None);
    }

    fn state(keys: &[KeyCode], bindings: &Bindings, ui_focus: bool) -> ActionState {
        let mut state = ActionState::default();
        state.update_for_test(bindings, keys, &[], ui_focus);
        state
    }

    fn gear_keys() -> Bindings {
        let mut bindings = Bindings::default();
        for (action, key) in [
            ("gear_reverse", "key:Digit0"),
            ("gear_neutral", "key:Digit9"),
            ("gear_1", "key:Digit1"),
            ("gear_3", "key:Digit3"),
            ("gear_7", "key:Digit7"),
        ] {
            bindings.bind(Action::parse(action).unwrap(), key, false).unwrap();
        }
        bindings
    }

    #[test]
    fn gear_keys_ask_for_their_gear_once() {
        let bindings = gear_keys();
        let mut actions = state(&[KeyCode::Digit3], &bindings, false);
        let wheel = WheelOptions::default();
        assert_eq!(DriveInput::from_actions(&actions, wheel).gear_select, Some(GEAR_FIRST + 2));
        actions.update_for_test(&bindings, &[KeyCode::Digit3], &[], false);
        assert_eq!(DriveInput::from_actions(&actions, wheel).gear_select, None, "held, it asks no more");
        let actions = state(&[KeyCode::Digit0], &bindings, false);
        assert_eq!(DriveInput::from_actions(&actions, wheel).gear_select, Some(GEAR_REVERSE));
        let actions = state(&[KeyCode::Digit9], &bindings, false);
        assert_eq!(DriveInput::from_actions(&actions, wheel).gear_select, Some(GEAR_NEUTRAL));
        let actions = state(&[KeyCode::Digit7], &bindings, false);
        assert_eq!(DriveInput::from_actions(&actions, wheel).gear_select, Some(GEAR_FIRST + 6));
        let actions = state(&[], &bindings, false);
        assert_eq!(DriveInput::from_actions(&actions, wheel).gear_select, None, "nothing pressed asks for nothing");
    }

    #[test]
    fn an_h_shifter_asks_for_the_gear_it_holds_and_neutral_in_the_gate() {
        let bindings = gear_keys();
        let wheel = WheelOptions { h_shifter: true, ..WheelOptions::default() };
        let held =
            |keys: &[KeyCode], ui_focus| DriveInput::from_actions(&state(keys, &bindings, ui_focus), wheel).gear_select;
        assert_eq!(held(&[KeyCode::Digit1], false), Some(GEAR_FIRST));
        assert_eq!(held(&[KeyCode::Digit1], false), Some(GEAR_FIRST), "and it keeps asking while held");
        assert_eq!(held(&[], false), Some(GEAR_NEUTRAL), "no button is the neutral gate");
        assert_eq!(held(&[KeyCode::Digit0], false), Some(GEAR_REVERSE));
        assert_eq!(held(&[KeyCode::Digit3, KeyCode::Digit7], false), Some(GEAR_FIRST + 2), "the lower gear wins");
        assert_eq!(held(&[], true), None, "typing in the console is not the driver shifting to neutral");
    }

    #[test]
    fn the_clutch_pedal_is_read_as_travel() {
        let mut bindings = Bindings::default();
        bindings.bind(Action::Clutch, "trigger:LeftTrigger", false).unwrap();
        let mut actions = ActionState::default();
        actions.update_for_test(&bindings, &[], &[(bevy_input::gamepad::GamepadButton::LeftTrigger, 0.4)], false);
        let input = DriveInput::from_actions(&actions, WheelOptions::default());
        assert!((input.clutch - 0.4).abs() < 1e-6);
        let released = DriveInput::from_actions(&ActionState::default(), WheelOptions::default());
        assert_eq!(released.clutch, 0.0);
    }

    #[test]
    fn shift_requests_fire_once_per_frame() {
        let input = DriveInput { throttle: 1.0, shift_up: true, ..DriveInput::default() };
        let later = input.held();
        assert!(!later.shift_up && later.throttle == 1.0);
    }
}
