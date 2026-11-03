//! Controller slots preserve other input families and represent steering axes once.
use super::{Action, Bindings, Control, Source, bindings::Binding, source};

fn owns(control: Control, binding: &Binding) -> bool {
    if binding.action != control.action || source::family(binding.source) != source::Family::Gamepad {
        return false;
    }
    if matches!(binding.source, Source::PadAxis(_)) && control.action == Action::Steer {
        return true;
    }
    control.direction.is_none_or(|direction| binding.scale.signum() == direction.signum())
}

fn same_source(a: Source, b: Source) -> bool {
    match (a, b) {
        (Source::PadButton(a) | Source::PadTrigger(a), Source::PadButton(b) | Source::PadTrigger(b)) => a == b,
        (
            Source::PadAxis(a) | Source::PedalAxis { axis: a, .. },
            Source::PadAxis(b) | Source::PedalAxis { axis: b, .. },
        ) => a == b,
        _ => a == b,
    }
}

fn driving_control(action: Action) -> bool {
    matches!(
        action,
        Action::Throttle
            | Action::Brake
            | Action::Steer
            | Action::Handbrake
            | Action::Speedbreaker
            | Action::Nos
            | Action::ShiftUp
            | Action::ShiftDown
            | Action::Clutch
            | Action::GearReverse
            | Action::GearNeutral
            | Action::Gear1
            | Action::Gear2
            | Action::Gear3
            | Action::Gear4
            | Action::Gear5
            | Action::Gear6
            | Action::Gear7
            | Action::ResetCar
    )
}

impl Bindings {
    pub fn gamepad_labels(&self, control: Control) -> Vec<String> {
        self.0
            .iter()
            .filter(|binding| owns(control, binding))
            .filter_map(|binding| super::describe::input_for(binding, control.direction.is_some_and(|d| d < 0.0)))
            .map(|name| name.strip_prefix("Pad ").unwrap_or(&name).to_uppercase())
            .collect()
    }

    /// `travel` is the observed input direction, so either steering row can capture a full signed axis.
    pub fn set_gamepad_slot(&mut self, control: Control, slot: usize, source: Source, travel: f32) {
        if slot >= super::slots::SLOTS
            || super::source::family(source) != source::Family::Gamepad
            || !travel.is_finite()
            || travel == 0.0
        {
            return;
        }
        let direction = control.direction.unwrap_or(1.0).signum();
        let scale = match source {
            Source::PadAxis(_) => direction * travel.signum(),
            _ => direction,
        };
        let mut own: Vec<_> = self.0.iter().filter(|binding| owns(control, binding)).copied().collect();
        let replacement = Binding { action: control.action, source, scale, per_second: false };
        let at = slot.min(own.len());
        if at == own.len() {
            own.push(replacement);
        }
        if at < own.len() {
            own[at] = replacement;
        }
        own = own
            .into_iter()
            .enumerate()
            .filter(|(index, binding)| *index == at || !same_source(binding.source, source))
            .map(|(_, binding)| binding)
            .collect();
        // Menu controls and the keyboard remain independent even when they share the same physical button.
        self.0.retain(|binding| {
            !owns(control, binding)
                && !((binding.action == control.action || driving_control(binding.action))
                    && same_source(binding.source, source))
        });
        self.0.extend(own);
    }
}

#[cfg(test)]
mod tests;
