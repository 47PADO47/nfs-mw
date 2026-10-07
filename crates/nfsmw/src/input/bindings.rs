//! Which device input drives which action. Defaults live here; a config file will override them.

use bevy_input::gamepad::{GamepadAxis, GamepadButton};
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::MouseButton;

use super::Action;
use super::snapshot::Snapshot;

/// When mouse motion counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// The cursor is captured, or the right button is held (free look).
    Look,
    /// Any of the left or right buttons is held (model orbiting).
    Drag,
}

impl Gate {
    fn open(self, s: &Snapshot) -> bool {
        let left = s.buttons.contains(&MouseButton::Left);
        let right = s.buttons.contains(&MouseButton::Right);
        match self {
            Gate::Look => s.mouse_captured || right,
            Gate::Drag => left || right,
        }
    }
}

/// One device input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Source {
    Key(KeyCode),
    /// Mouse motion along x (`false`) or y (`true`), while the gate is open.
    MouseMotion {
        y: bool,
        gate: Gate,
    },
    Scroll,
    PadAxis(GamepadAxis),
    PadButton(GamepadButton),
}

/// `source` adds `scale` (× the seconds of the frame, if `per_second`) to `action`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Binding {
    pub action: Action,
    pub source: Source,
    pub scale: f32,
    pub per_second: bool,
}

/// Sticks within this distance of the centre read as zero.
pub const DEADZONE: f32 = 0.15;

impl Binding {
    const fn new(action: Action, source: Source, scale: f32) -> Self {
        Self { action, source, scale, per_second: false }
    }

    const fn per_second(action: Action, source: Source, scale: f32) -> Self {
        Self { action, source, scale, per_second: true }
    }

    /// What this binding contributes to its action now.
    pub(super) fn value(&self, s: &Snapshot) -> f32 {
        let raw = match self.source {
            Source::Key(k) => f32::from(u8::from(s.keys.contains(&k))),
            Source::MouseMotion { y, gate } if gate.open(s) => {
                if y {
                    s.mouse_delta.1
                } else {
                    s.mouse_delta.0
                }
            }
            Source::MouseMotion { .. } => 0.0,
            Source::Scroll => s.scroll,
            Source::PadAxis(a) => deadzone(s.pad_axis(a)),
            Source::PadButton(b) => f32::from(u8::from(s.pad_buttons.contains(&b))),
        };
        raw * self.scale * if self.per_second { s.dt } else { 1.0 }
    }
}

/// Rescale so the output still reaches 1 at full deflection.
fn deadzone(v: f32) -> f32 {
    if v.abs() < DEADZONE { 0.0 } else { v.signum() * (v.abs() - DEADZONE) / (1.0 - DEADZONE) }
}

/// Mouse pixels per second a fully pushed stick turns the camera by.
const STICK_LOOK_PIXELS: f32 = 700.0;
/// Scroll lines per second while a button is held.
const BUTTON_ZOOM_LINES: f32 = 6.0;

/// The default layout: keyboard and mouse, plus a standard twin-stick gamepad.
pub fn defaults() -> Vec<Binding> {
    use Action::*;
    let key = Source::Key;
    let motion = |y, gate| Source::MouseMotion { y, gate };
    let pad = Source::PadAxis;
    let button = Source::PadButton;
    vec![
        // Keyboard movement.
        Binding::new(MoveForward, key(KeyCode::KeyW), 1.0),
        Binding::new(MoveForward, key(KeyCode::KeyS), -1.0),
        Binding::new(MoveRight, key(KeyCode::KeyD), 1.0),
        Binding::new(MoveRight, key(KeyCode::KeyA), -1.0),
        Binding::new(MoveUp, key(KeyCode::Space), 1.0),
        Binding::new(MoveUp, key(KeyCode::KeyE), 1.0),
        Binding::new(MoveUp, key(KeyCode::KeyC), -1.0),
        Binding::new(MoveUp, key(KeyCode::KeyQ), -1.0),
        Binding::new(Boost, key(KeyCode::ShiftLeft), 1.0),
        Binding::new(Boost, key(KeyCode::ShiftRight), 1.0),
        Binding::new(Cancel, key(KeyCode::Escape), 1.0),
        Binding::new(Console, key(KeyCode::F12), 1.0),
        // Mouse.
        Binding::new(LookX, motion(false, Gate::Look), 1.0),
        Binding::new(LookY, motion(true, Gate::Look), 1.0),
        Binding::new(OrbitX, motion(false, Gate::Drag), 1.0),
        Binding::new(OrbitY, motion(true, Gate::Drag), 1.0),
        Binding::new(Zoom, Source::Scroll, 1.0),
        // Gamepad: left stick moves, right stick looks, triggers go up and down.
        Binding::new(MoveForward, pad(GamepadAxis::LeftStickY), 1.0),
        Binding::new(MoveRight, pad(GamepadAxis::LeftStickX), 1.0),
        Binding::new(MoveUp, button(GamepadButton::South), 1.0),
        Binding::new(MoveUp, button(GamepadButton::East), -1.0),
        Binding::new(Boost, button(GamepadButton::LeftThumb), 1.0),
        Binding::new(Boost, button(GamepadButton::RightTrigger), 1.0),
        Binding::new(Cancel, button(GamepadButton::Start), 1.0),
        Binding::per_second(LookX, pad(GamepadAxis::RightStickX), STICK_LOOK_PIXELS),
        Binding::per_second(LookY, pad(GamepadAxis::RightStickY), -STICK_LOOK_PIXELS),
        Binding::per_second(OrbitX, pad(GamepadAxis::RightStickX), STICK_LOOK_PIXELS),
        Binding::per_second(OrbitY, pad(GamepadAxis::RightStickY), -STICK_LOOK_PIXELS),
        Binding::per_second(Zoom, button(GamepadButton::DPadUp), BUTTON_ZOOM_LINES),
        Binding::per_second(Zoom, button(GamepadButton::DPadDown), -BUTTON_ZOOM_LINES),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_rescales() {
        assert_eq!(deadzone(0.1), 0.0);
        assert_eq!(deadzone(-0.1), 0.0);
        assert!((deadzone(1.0) - 1.0).abs() < 1e-6);
        assert!((deadzone(-1.0) + 1.0).abs() < 1e-6);
        assert!(deadzone(0.5) > 0.0 && deadzone(0.5) < 0.5);
    }

    #[test]
    fn mouse_motion_waits_for_its_gate() {
        let look = Binding::new(Action::LookX, Source::MouseMotion { y: false, gate: Gate::Look }, 1.0);
        let mut s = Snapshot { mouse_delta: (4.0, 0.0), ..Snapshot::default() };
        assert_eq!(look.value(&s), 0.0);
        s.buttons.insert(MouseButton::Right);
        assert_eq!(look.value(&s), 4.0);
        s.buttons.clear();
        s.mouse_captured = true;
        assert_eq!(look.value(&s), 4.0);

        let orbit = Binding::new(Action::OrbitX, Source::MouseMotion { y: false, gate: Gate::Drag }, 1.0);
        assert_eq!(orbit.value(&s), 0.0, "capture alone does not orbit");
        s.buttons.insert(MouseButton::Left);
        assert_eq!(orbit.value(&s), 4.0);
    }

    #[test]
    fn stick_turns_per_second() {
        let b = Binding::per_second(Action::LookX, Source::PadAxis(GamepadAxis::RightStickX), 700.0);
        let mut s = Snapshot { dt: 0.5, ..Snapshot::default() };
        s.pad_axes.insert(GamepadAxis::RightStickX, 1.0);
        assert!((b.value(&s) - 350.0).abs() < 1e-3);
    }
}
