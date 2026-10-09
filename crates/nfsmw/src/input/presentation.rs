//! Device ownership and prompt selection. Neutral polling never changes the visible device.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use bevy_input::gamepad::{Gamepad, GamepadInput};

use super::{Action, Bindings, bindings::Source};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputDevice {
    #[default]
    Keyboard,
    Xbox,
}

#[derive(Resource, Default, Debug)]
pub struct InputPresentation {
    pub device: InputDevice,
    pub active_pad: Option<Entity>,
    /// One frame only, used to pause driving when its controller is unplugged.
    pub disconnected: bool,
    initialized: bool,
    previous: HashMap<(Entity, GamepadInput), f32>,
}

impl InputPresentation {
    pub fn sample_pads(&mut self, pads: &Query<(Entity, &Gamepad)>, bindings: &Bindings) {
        self.disconnected = self.active_pad.is_some_and(|e| pads.get(e).is_err());
        if self.disconnected {
            self.active_pad = None;
            self.device = InputDevice::Keyboard;
        }
        self.previous.retain(|(e, _), _| pads.get(*e).is_ok());
        let first = pads.iter().map(|(e, _)| e).min();
        if !self.initialized && first.is_some() {
            self.active_pad = first;
            self.device = InputDevice::Xbox;
        }
        self.initialized = true;
        let mut activity = Vec::new();
        for (entity, pad) in pads {
            let button = pad.get_just_pressed().any(|button| {
                bindings.0.iter().any(|b| b.scale != 0.0 && matches!(b.source, Source::PadButton(v) if v == *button))
            });
            let mut analog = false;
            for input in pad.get_analog_axes() {
                let value = pad.get(*input).unwrap_or(0.0);
                let anchor = self.previous.entry((entity, *input)).or_insert(0.0);
                let bound = bindings.0.iter().filter(|b| b.scale != 0.0).any(|b| match (b.source, input) {
                    (Source::PadAxis(a), GamepadInput::Axis(v)) => a == *v,
                    (Source::PadTrigger(a), GamepadInput::Button(v)) => a == *v,
                    _ => false,
                });
                let threshold = match input {
                    GamepadInput::Axis(_) => 0.55,
                    GamepadInput::Button(_) => 0.25,
                };
                // Compare with the last meaningful position, not the preceding sample: deliberate
                // slow movement must take ownership too, while small held-stick noise stays quiet.
                let moved = bound && value.abs() >= threshold && (value - *anchor).abs() >= 0.15;
                analog |= moved;
                if moved || value.abs() <= threshold * 0.5 {
                    *anchor = value;
                }
            }
            if button || analog {
                activity.push(entity);
            }
        }
        // Deterministic tie break when two controllers act in the same frame.
        if let Some(entity) = activity.into_iter().min() {
            self.active_pad = Some(entity);
            self.device = InputDevice::Xbox;
        }
        if self.active_pad.is_none() {
            self.active_pad = first;
        }
    }

    pub fn keyboard_activity(&mut self) {
        self.device = InputDevice::Keyboard;
    }
}

impl Bindings {
    /// The first nonzero binding in this device family, retaining the scale for signed axes.
    pub fn prompt_binding(&self, action: Action, device: InputDevice) -> Option<&super::bindings::Binding> {
        self.0.iter().find(|b| {
            if b.action != action || b.scale == 0.0 {
                return false;
            }
            let gamepad = matches!(b.source, Source::PadAxis(_) | Source::PadButton(_) | Source::PadTrigger(_));
            gamepad == (device == InputDevice::Xbox)
        })
    }
}
