//! Physical controller capture is modal and waits for neutral input before accepting a new gesture.
use super::{flow::Frontend, ids::screen};
use crate::input::{Bindings, Control, Source};
use bevy_ecs::prelude::*;
use bevy_input::{
    ButtonInput,
    gamepad::{Gamepad, GamepadAxis, GamepadButton, GamepadInput},
    keyboard::KeyCode,
};

#[derive(Default)]
struct Physical {
    buttons: Vec<GamepadButton>,
    axes: Vec<(GamepadAxis, f32)>,
    triggers: Vec<(GamepadButton, f32)>,
}

impl Physical {
    fn read(pad: &Gamepad) -> Self {
        let mut sample = Self { buttons: pad.get_pressed().copied().collect(), ..Default::default() };
        for input in pad.get_analog_axes() {
            let Some(value) = pad.get(*input).filter(|value| value.is_finite()) else { continue };
            match *input {
                GamepadInput::Axis(axis) => sample.axes.push((axis, value)),
                GamepadInput::Button(button)
                    if matches!(button, GamepadButton::LeftTrigger2 | GamepadButton::RightTrigger2) =>
                {
                    sample.triggers.push((button, value))
                }
                GamepadInput::Button(button) if value >= 0.55 && !sample.buttons.contains(&button) => {
                    sample.buttons.push(button)
                }
                _ => {}
            }
        }
        sample
    }

    fn neutral(&self) -> bool {
        self.buttons.is_empty()
            && self.axes.iter().all(|(_, value)| value.abs() < 0.2)
            && self.triggers.iter().all(|(_, value)| *value < 0.2)
    }

    fn candidate(&self) -> Option<(Source, f32)> {
        // Trigger analog and digital reports describe one physical source.
        if let Some((button, value)) = self.triggers.iter().find(|(_, value)| *value >= 0.55) {
            return Some((Source::PadTrigger(*button), *value));
        }
        if let Some(button) = self.buttons.first() {
            let source = match button {
                GamepadButton::LeftTrigger2 | GamepadButton::RightTrigger2 => Source::PadTrigger(*button),
                _ => Source::PadButton(*button),
            };
            return Some((source, 1.0));
        }
        self.axes
            .iter()
            .filter(|(_, value)| value.abs() >= 0.65)
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .map(|(axis, value)| (Source::PadAxis(*axis), *value))
    }
}

pub(super) struct Capture {
    pub control: Control,
    pub slot: usize,
    pad: Option<Entity>,
    armed: bool,
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Waiting,
    Cancelled,
    Bound(Source, f32),
}

impl Capture {
    pub fn new(control: Control, slot: usize) -> Self {
        Self { control, slot, pad: None, armed: false }
    }

    pub fn hint(&self) -> String {
        let key = match (self.pad.is_some(), self.armed) {
            (false, _) => "controls-connect-controller",
            (true, false) => "controls-release-input",
            (true, true) => "controls-waiting-input",
        };
        crate::i18n::text(key, &crate::i18n::Args::new()).to_uppercase()
    }

    fn sample(&mut self, entity: Entity, sample: &Physical) -> Option<(Source, f32)> {
        if self.pad.is_some_and(|wanted| wanted != entity) {
            return None;
        }
        self.pad = Some(entity);
        if !self.armed {
            self.armed = sample.neutral();
            return None;
        }
        sample.candidate()
    }

    fn step(&mut self, entity: Option<Entity>, sample: Option<&Physical>, escape: bool) -> Outcome {
        if escape {
            return Outcome::Cancelled;
        }
        let (Some(entity), Some(sample)) = (entity, sample) else {
            self.pad = None;
            self.armed = false;
            return Outcome::Waiting;
        };
        match self.sample(entity, sample) {
            Some((source, travel)) => Outcome::Bound(source, travel),
            None => Outcome::Waiting,
        }
    }
}

pub(super) fn begin(fe: &mut Frontend, control: Control, slot: usize, controller: bool) {
    fe.remap = None;
    fe.controller_remap = None;
    if controller {
        fe.controller_remap = Some(Capture::new(control, slot));
        return;
    }
    fe.remap = Some((control, slot));
}

/// Returns true for every capture frame, including completion/cancellation, to consume that physical gesture.
pub(super) fn update(
    fe: &mut Frontend,
    bindings: &mut Bindings,
    keys: &ButtonInput<KeyCode>,
    scripted_key: Option<KeyCode>,
    active: Option<Entity>,
    pads: &Query<(Entity, &Gamepad)>,
) -> bool {
    if !fe.screens.top().is_some_and(|name| [screen::CONTROLS, screen::PAUSE_CONTROLS].contains(&name)) {
        fe.remap = None;
        fe.controller_remap = None;
        return false;
    }
    let capturing = fe.remap.is_some() || fe.controller_remap.is_some();
    if let Some((control, slot)) = fe.remap
        && let Some(key) = scripted_key.or_else(|| keys.get_just_pressed().next().copied())
    {
        fe.remap = None;
        if key != KeyCode::Escape {
            super::plugin_keys::remember_key(bindings, control, slot, key, fe.save);
        }
    }
    let escape = scripted_key == Some(KeyCode::Escape) || keys.just_pressed(KeyCode::Escape);
    if let Some(capture) = &mut fe.controller_remap {
        let selected = capture.pad.or(active);
        let physical = selected.and_then(|entity| pads.get(entity).ok()).map(|(_, pad)| Physical::read(pad));
        match capture.step(selected, physical.as_ref(), escape) {
            Outcome::Waiting => {}
            Outcome::Cancelled => fe.controller_remap = None,
            Outcome::Bound(source, travel) => {
                bindings.set_gamepad_slot(capture.control, capture.slot, source, travel);
                super::plugin_keys::save_bindings(bindings, fe.save);
                fe.controller_remap = None;
            }
        }
    }
    capturing
}

#[cfg(test)]
mod tests;
