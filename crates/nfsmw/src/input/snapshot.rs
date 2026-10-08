//! The raw device state for one frame, as the bindings see it.

use std::collections::{HashMap, HashSet};

use bevy_input::gamepad::{GamepadAxis, GamepadButton};
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::MouseButton;

/// Everything the bindings read. Built from Bevy's input resources each frame (see `systems`), so the
/// resolving code has no Bevy system parameters and can be tested directly.
#[derive(Debug, Default, Clone)]
pub struct Snapshot {
    pub keys: HashSet<KeyCode>,
    pub buttons: HashSet<MouseButton>,
    /// Mouse motion since the last frame, in pixels.
    pub mouse_delta: (f32, f32),
    /// Scroll since the last frame, in lines.
    pub scroll: f32,
    /// Analog axes, -1..1, from all connected gamepads (the largest value per axis).
    pub pad_axes: HashMap<GamepadAxis, f32>,
    pub pad_buttons: HashSet<GamepadButton>,
    /// Analog pull of the buttons that report one (the triggers), 0..1; the largest per button.
    pub pad_triggers: HashMap<GamepadButton, f32>,
    /// The cursor is captured for mouse look.
    pub mouse_captured: bool,
    /// Seconds since the last frame.
    pub dt: f32,
}

impl Snapshot {
    pub(super) fn pad_axis(&self, axis: GamepadAxis) -> f32 {
        self.pad_axes.get(&axis).copied().unwrap_or(0.0)
    }

    /// 1 when the button is held, else 0.
    pub(super) fn pad_button_value(&self, button: GamepadButton) -> f32 {
        f32::from(u8::from(self.pad_buttons.contains(&button)))
    }
}
