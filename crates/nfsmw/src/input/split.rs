//! Two players on one machine (split screen): who holds which device, and the second player's actions.
//!
//! The first player's actions are the ordinary [`ActionState`](super::ActionState); the layer only narrows what
//! feeds it while a split-screen race is on (the keyboard keys and the pad of the other player stay out). The second
//! player's actions are [`SplitInput::second`], resolved from the second device's own bindings.
//! Spec: `docs/specs/split-screen.md` section 5.

use std::collections::HashSet;

use bevy_ecs::prelude::{Entity, Resource};
use bevy_input::gamepad::GamepadButton;
use bevy_input::keyboard::KeyCode;

use super::Action;
use super::bindings::{self, Binding, Source};
use super::snapshot::Snapshot;
use super::state::{ActionState, Bindings};

/// A device a split-screen player holds. The original maps each player to a controller port; a PC has the keyboard and
/// any number of pads, so a player holds one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerDevice {
    /// The keyboard with the ordinary bindings (WASD, Space, Left Shift, Enter...).
    Keyboard,
    /// The arrow block and the keys beside it, for a second player sharing the keyboard.
    KeyboardArrows,
    /// One gamepad, by its entity.
    Pad(Entity),
}

/// Keys that stay with the first player when a pad holds their seat: the system keys (back out, console, function keys).
const SYSTEM_KEYS: [KeyCode; 13] = [
    KeyCode::Escape,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::F12,
];

/// The keys the arrow-block player owns: the first player's keyboard loses them while that player shares the keyboard.
const ARROW_KEYS: [KeyCode; 11] = [
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ShiftRight,
    KeyCode::ControlRight,
    KeyCode::Numpad0,
    KeyCode::PageUp,
    KeyCode::PageDown,
    KeyCode::Delete,
    KeyCode::End,
];

/// What a split-screen race adds to the input layer.
#[derive(Resource, Debug, Clone, Default)]
pub struct SplitInput {
    devices: Option<[PlayerDevice; 2]>,
    /// The second player's actions, resolved every frame while a split-screen race is on.
    pub second: ActionState,
    /// The device whose Start (or accept) key was pressed this frame: the "press start" screens read it to seat a player.
    pub start_press: Option<PlayerDevice>,
}

impl SplitInput {
    /// Seat the two players (the front end does this when the race starts) or clear the seats (`None`).
    pub fn seat(&mut self, devices: Option<[PlayerDevice; 2]>) {
        self.devices = devices;
        self.second = ActionState::default();
    }

    /// The devices of player 1 and player 2 while a split-screen race is on.
    pub fn devices(&self) -> Option<[PlayerDevice; 2]> {
        self.devices
    }

    /// A seated pad that is no longer connected, given the pads that are.
    pub fn lost_pad(&self, connected: &HashSet<Entity>) -> Option<usize> {
        let devices = self.devices?;
        devices.iter().position(|d| matches!(d, PlayerDevice::Pad(pad) if !connected.contains(pad)))
    }

    /// The first player's snapshot: the keys and the pad of their own device only. `pads` gives the pad's state by entity.
    pub fn first_snapshot(&self, all: &Snapshot) -> Snapshot {
        let Some([first, second]) = self.devices else { return all.clone() };
        let mut snapshot = all.clone();
        snapshot.keys = keys_for(first, second, &all.keys);
        snapshot
    }

    /// The second player's snapshot: only their own keys; the pad state is filled by the caller from their pad.
    pub fn second_snapshot(&self, all: &Snapshot) -> Snapshot {
        let Some([first, second]) = self.devices else { return Snapshot::default() };
        Snapshot { controls: all.controls, keys: keys_for(second, first, &all.keys), dt: all.dt, ..Snapshot::default() }
    }

    /// The pad a player holds, if any.
    pub fn pad_of(&self, player: usize) -> Option<Entity> {
        match self.devices?.get(player)? {
            PlayerDevice::Pad(pad) => Some(*pad),
            _ => None,
        }
    }

    /// Resolve the second player's actions from their snapshot.
    pub fn update_second(&mut self, snapshot: &Snapshot) {
        let Some([_, second]) = self.devices else { return };
        self.second.update(&second_bindings(second), snapshot, false);
    }
}

/// The keys `own` gets of `all` while `other` holds the other seat.
fn keys_for(own: PlayerDevice, other: PlayerDevice, all: &HashSet<KeyCode>) -> HashSet<KeyCode> {
    match (own, other) {
        (PlayerDevice::Pad(_), _) => all.iter().copied().filter(|k| SYSTEM_KEYS.contains(k)).collect(),
        (PlayerDevice::KeyboardArrows, _) => all.iter().copied().filter(|k| ARROW_KEYS.contains(k)).collect(),
        (PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows) => {
            all.iter().copied().filter(|k| !ARROW_KEYS.contains(k)).collect()
        }
        (PlayerDevice::Keyboard, _) => all.clone(),
    }
}

/// The second player's bindings: the ordinary ones for a pad or the keyboard, the arrow block's own for a shared keyboard.
fn second_bindings(device: PlayerDevice) -> Bindings {
    match device {
        PlayerDevice::KeyboardArrows => Bindings(arrow_bindings()),
        _ => Bindings(bindings::defaults()),
    }
}

/// The arrow-block layout: arrows drive, Right Shift is nitrous, Numpad 0 or Right Ctrl the handbrake, Page Up and Page
/// Down shift, Delete resets the car, End looks back.
fn arrow_bindings() -> Vec<Binding> {
    use Action::*;
    let key = Source::Key;
    let entries = [
        (Throttle, KeyCode::ArrowUp, 1.0),
        (Brake, KeyCode::ArrowDown, 1.0),
        (Steer, KeyCode::ArrowRight, 1.0),
        (Steer, KeyCode::ArrowLeft, -1.0),
        (Handbrake, KeyCode::Numpad0, 1.0),
        (Handbrake, KeyCode::ControlRight, 1.0),
        (Nos, KeyCode::ShiftRight, 1.0),
        (ShiftUp, KeyCode::PageUp, 1.0),
        (ShiftDown, KeyCode::PageDown, 1.0),
        (ResetCar, KeyCode::Delete, 1.0),
        (LookBack, KeyCode::End, 1.0),
    ];
    entries.into_iter().map(|(action, code, scale)| Binding::new(action, key(code), scale)).collect()
}

/// The device whose Start or accept key went down this frame, from the keys and pad buttons just pressed. The keyboard's
/// Enter or Space seats the ordinary keyboard; Numpad Enter or Numpad 0 the arrow block; a pad's Start or A that pad.
pub fn start_press(
    just_keys: &HashSet<KeyCode>,
    just_pads: impl IntoIterator<Item = (Entity, HashSet<GamepadButton>)>,
) -> Option<PlayerDevice> {
    let pad = just_pads
        .into_iter()
        .filter(|(_, buttons)| buttons.contains(&GamepadButton::Start) || buttons.contains(&GamepadButton::South))
        .map(|(entity, _)| entity)
        .min();
    if let Some(pad) = pad {
        return Some(PlayerDevice::Pad(pad));
    }
    if just_keys.contains(&KeyCode::NumpadEnter) || just_keys.contains(&KeyCode::Numpad0) {
        return Some(PlayerDevice::KeyboardArrows);
    }
    if just_keys.contains(&KeyCode::Enter) || just_keys.contains(&KeyCode::Space) {
        return Some(PlayerDevice::Keyboard);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(n: u32) -> Entity {
        Entity::from_raw_u32(n).unwrap()
    }

    fn keys(codes: &[KeyCode]) -> HashSet<KeyCode> {
        codes.iter().copied().collect()
    }

    #[test]
    fn a_shared_keyboard_splits_between_the_players() {
        let mut input = SplitInput::default();
        input.seat(Some([PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows]));
        let all = Snapshot { keys: keys(&[KeyCode::KeyW, KeyCode::ArrowUp, KeyCode::Escape]), ..Snapshot::default() };
        let first = input.first_snapshot(&all);
        let second = input.second_snapshot(&all);
        assert!(first.keys.contains(&KeyCode::KeyW) && first.keys.contains(&KeyCode::Escape));
        assert!(!first.keys.contains(&KeyCode::ArrowUp), "the arrows are the second player's");
        assert_eq!(second.keys, keys(&[KeyCode::ArrowUp]));
    }

    #[test]
    fn the_second_player_drives_with_the_arrows() {
        let mut input = SplitInput::default();
        input.seat(Some([PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows]));
        let all = Snapshot {
            keys: keys(&[KeyCode::ArrowUp, KeyCode::ArrowLeft, KeyCode::ShiftRight]),
            ..Snapshot::default()
        };
        let second = input.second_snapshot(&all);
        input.update_second(&second);
        assert!(input.second.pressed(Action::Throttle));
        assert!(input.second.pressed(Action::Nos));
        assert!((input.second.value(Action::Steer) + 1.0).abs() < f32::EPSILON);
        assert!(!input.second.pressed(Action::Brake));
    }

    #[test]
    fn a_pad_player_leaves_the_keyboard_to_the_other() {
        let mut input = SplitInput::default();
        input.seat(Some([PlayerDevice::Pad(pad(1)), PlayerDevice::Keyboard]));
        let all = Snapshot { keys: keys(&[KeyCode::KeyW, KeyCode::Escape]), ..Snapshot::default() };
        assert_eq!(input.first_snapshot(&all).keys, keys(&[KeyCode::Escape]), "only the system keys stay");
        assert_eq!(input.second_snapshot(&all).keys, keys(&[KeyCode::KeyW, KeyCode::Escape]));
        assert_eq!(input.pad_of(0), Some(pad(1)));
        assert_eq!(input.pad_of(1), None);
    }

    #[test]
    fn without_seats_the_first_snapshot_is_unchanged() {
        let input = SplitInput::default();
        let all = Snapshot { keys: keys(&[KeyCode::KeyW, KeyCode::ArrowUp]), ..Snapshot::default() };
        assert_eq!(input.first_snapshot(&all).keys, all.keys);
        assert!(input.second_snapshot(&all).keys.is_empty());
    }

    #[test]
    fn a_vanished_pad_is_reported() {
        let mut input = SplitInput::default();
        input.seat(Some([PlayerDevice::Keyboard, PlayerDevice::Pad(pad(2))]));
        assert_eq!(input.lost_pad(&[pad(2)].into_iter().collect()), None);
        assert_eq!(input.lost_pad(&HashSet::new()), Some(1));
    }

    #[test]
    fn start_seats_the_device_that_pressed_it() {
        let none = HashSet::new();
        assert_eq!(start_press(&keys(&[KeyCode::Enter]), []), Some(PlayerDevice::Keyboard));
        assert_eq!(start_press(&keys(&[KeyCode::NumpadEnter]), []), Some(PlayerDevice::KeyboardArrows));
        let start: HashSet<_> = [GamepadButton::Start].into_iter().collect();
        let forward = start_press(&none, [(pad(3), start.clone()), (pad(2), start.clone())]);
        let backward = start_press(&none, [(pad(2), start.clone()), (pad(3), start)]);
        assert!(matches!(forward, Some(PlayerDevice::Pad(_))));
        assert_eq!(forward, backward, "two pads in one frame: the same one wins whatever the order");
        assert_eq!(start_press(&keys(&[KeyCode::KeyW]), []), None);
    }
}
