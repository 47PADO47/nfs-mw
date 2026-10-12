//! Keyboard slots: the primary and secondary key of a control, as the controls screen shows and changes them.
//! A control is an action, or one direction of an axis (Steer left and Steer right are the same action, told apart
//! by the sign of the key's scale). The keyboard is one device family, so a control's slots are its keyboard bindings
//! in file order: slot 0 is the primary key and slot 1 the secondary (docs/specs/controls-remap.md).

#![allow(dead_code)] // the controls screen that uses these is not wired to its package yet

use bevy_input::keyboard::KeyCode;

use super::Action;
use super::bindings::{Binding, Source};
use super::state::Bindings;

/// The keyboard slots the controls screen shows for each control.
pub const SLOTS: usize = 2;

/// A row of the controls screen: an action, and for an axis the direction (`Some(1.0)` right or forward, `Some(-1.0)`
/// left or back). `None` takes every keyboard binding of the action (a button).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Control {
    pub action: Action,
    pub direction: Option<f32>,
}

impl Control {
    pub const fn button(action: Action) -> Self {
        Self { action, direction: None }
    }

    pub const fn axis(action: Action, direction: f32) -> Self {
        Self { action, direction: Some(direction) }
    }

    /// Whether a binding belongs to this control: a keyboard key of the action, in this direction.
    fn owns(&self, binding: &Binding) -> bool {
        if binding.action != self.action || key_of(binding).is_none() {
            return false;
        }
        match self.direction {
            None => true,
            Some(d) => binding.scale.signum() == d.signum(),
        }
    }

    /// The scale a new key gets when the slot has none yet.
    fn default_scale(&self) -> f32 {
        self.direction.unwrap_or(1.0)
    }
}

impl Bindings {
    /// The keys of a control, in file order (slot 0 first).
    pub fn keyboard_keys(&self, control: Control) -> Vec<KeyCode> {
        self.0.iter().filter(|b| control.owns(b)).filter_map(key_of).collect()
    }

    /// Puts `key` in a slot of a control. The slot keeps the scale of the binding it replaces (an `S` that moves
    /// backwards keeps its minus when another key takes its place), and a new slot takes the control's direction.
    /// The key is taken away from every other binding, so one key drives one thing.
    pub fn set_keyboard_slot(&mut self, control: Control, slot: usize, key: KeyCode) {
        if slot >= SLOTS {
            return;
        }
        let mut own = own_bindings(self, control);
        let scale = own.get(slot).map_or(control.default_scale(), |b| b.scale);
        let fresh = Binding { action: control.action, source: Source::Key(key), scale, per_second: false };
        if slot < own.len() {
            own[slot] = fresh;
        } else {
            own.push(fresh);
        }
        // The key sits in one slot of this control only.
        let own: Vec<Binding> =
            own.into_iter().enumerate().filter(|(i, b)| *i == slot || key_of(b) != Some(key)).map(|(_, b)| b).collect();
        self.0.retain(|b| !control.owns(b) && key_of(b) != Some(key));
        self.0.extend(own);
    }

    /// Empties a slot of a control. The slots after it move up, as the screen lists them.
    pub fn clear_keyboard_slot(&mut self, control: Control, slot: usize) {
        let mut own = own_bindings(self, control);
        if slot >= own.len() {
            return;
        }
        own.remove(slot);
        self.0.retain(|b| !control.owns(b));
        self.0.extend(own);
    }
}

fn key_of(binding: &Binding) -> Option<KeyCode> {
    match binding.source {
        Source::Key(key) => Some(key),
        _ => None,
    }
}

/// The keyboard bindings of a control, in file order.
fn own_bindings(bindings: &Bindings, control: Control) -> Vec<Binding> {
    bindings.0.iter().filter(|b| control.owns(b)).copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(b: &Bindings, c: Control) -> Vec<KeyCode> {
        b.keyboard_keys(c)
    }

    const FORWARD: Control = Control::button(Action::MoveForward);

    #[test]
    fn the_defaults_show_the_primary_and_secondary_keys() {
        // MoveForward is W (+1) then S (-1) in the defaults.
        assert_eq!(keys(&Bindings::default(), FORWARD), vec![KeyCode::KeyW, KeyCode::KeyS]);
    }

    #[test]
    fn a_replaced_slot_keeps_the_sign_of_the_key_it_replaces() {
        let mut b = Bindings::default();
        b.set_keyboard_slot(FORWARD, 1, KeyCode::KeyK);
        assert_eq!(keys(&b, FORWARD), vec![KeyCode::KeyW, KeyCode::KeyK]);
        let scale = b.0.iter().find(|x| x.source == Source::Key(KeyCode::KeyK)).unwrap().scale;
        assert_eq!(scale, -1.0);
    }

    #[test]
    fn the_two_directions_of_an_axis_are_separate_controls() {
        let left = Control::axis(Action::Steer, -1.0);
        let right = Control::axis(Action::Steer, 1.0);
        let mut b = Bindings::default();
        let before = keys(&b, right);
        b.set_keyboard_slot(left, 0, KeyCode::KeyJ);
        assert_eq!(keys(&b, left)[0], KeyCode::KeyJ);
        assert_eq!(keys(&b, right), before, "the right steer keys stay as they were");
    }

    #[test]
    fn a_key_moves_away_from_the_control_that_had_it() {
        let mut b = Bindings::default();
        // Space jumps in the defaults (MoveUp); giving it to Boost takes it away from MoveUp.
        b.set_keyboard_slot(Control::button(Action::Boost), 0, KeyCode::Space);
        assert!(!keys(&b, Control::button(Action::MoveUp)).contains(&KeyCode::Space));
        assert_eq!(keys(&b, Control::button(Action::Boost))[0], KeyCode::Space);
    }

    #[test]
    fn one_key_is_never_in_both_slots_of_a_control() {
        let boost = Control::button(Action::Boost);
        let mut b = Bindings(Vec::new());
        b.set_keyboard_slot(boost, 0, KeyCode::KeyB);
        b.set_keyboard_slot(boost, 1, KeyCode::KeyB);
        assert_eq!(keys(&b, boost), vec![KeyCode::KeyB]);
    }

    #[test]
    fn clearing_a_slot_moves_the_next_one_up() {
        let boost = Control::button(Action::Boost);
        let mut b = Bindings(Vec::new());
        b.set_keyboard_slot(boost, 0, KeyCode::KeyA);
        b.set_keyboard_slot(boost, 1, KeyCode::KeyB);
        b.clear_keyboard_slot(boost, 0);
        assert_eq!(keys(&b, boost), vec![KeyCode::KeyB]);
    }

    #[test]
    fn a_slot_past_the_end_is_refused() {
        let boost = Control::button(Action::Boost);
        let mut b = Bindings(Vec::new());
        b.set_keyboard_slot(boost, 2, KeyCode::KeyA);
        assert!(keys(&b, boost).is_empty());
    }
}
