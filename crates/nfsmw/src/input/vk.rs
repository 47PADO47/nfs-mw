//! Windows virtual-key codes, as the Extra Options file names its hotkeys, mapped to the keys this port
//! binds. Only the keys a player would plausibly use; any other code is unknown.

use bevy_input::keyboard::KeyCode;

const LETTERS: [KeyCode; 26] = [
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
];

const DIGITS: [KeyCode; 10] = [
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

const FUNCTION: [KeyCode; 12] = [
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

/// The key for a Windows virtual-key code, or `None` when this port does not bind it by name.
pub fn key_for_vk(code: u32) -> Option<KeyCode> {
    let named = match code {
        0x08 => KeyCode::Backspace,
        0x09 => KeyCode::Tab,
        0x0D => KeyCode::Enter,
        0x13 => KeyCode::Pause,
        0x1B => KeyCode::Escape,
        0x20 => KeyCode::Space,
        0x21 => KeyCode::PageUp,
        0x22 => KeyCode::PageDown,
        0x23 => KeyCode::End,
        0x24 => KeyCode::Home,
        0x2D => KeyCode::Insert,
        0x2E => KeyCode::Delete,
        0x91 => KeyCode::ScrollLock,
        0xA0 => KeyCode::ShiftLeft,
        0xA1 => KeyCode::ShiftRight,
        0xA2 => KeyCode::ControlLeft,
        0xA3 => KeyCode::ControlRight,
        _ => return None,
    };
    Some(named)
}

/// The digit, letter and function keys by their code (`0x30..0x39`, `0x41..0x5A`, `0x70..0x7B`).
fn table_key(code: u32) -> Option<KeyCode> {
    match code {
        0x30..=0x39 => DIGITS.get((code - 0x30) as usize).copied(),
        0x41..=0x5A => LETTERS.get((code - 0x41) as usize).copied(),
        0x70..=0x7B => FUNCTION.get((code - 0x70) as usize).copied(),
        _ => None,
    }
}

/// Like [`key_for_vk`], and the digit, letter and function keys too.
pub fn key(code: u32) -> Option<KeyCode> {
    key_for_vk(code).or_else(|| table_key(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mods_default_hotkeys_map_to_their_keys() {
        assert_eq!(key(72), Some(KeyCode::KeyH), "Headlights = 72 is H");
        assert_eq!(key(33), Some(KeyCode::PageUp), "ChangeHeatLevel = 33 is PageUp");
        assert_eq!(key(115), Some(KeyCode::F4), "FreezeCar = 115 is F4");
        assert_eq!(key(145), Some(KeyCode::ScrollLock), "ToggleCops = 145 is Scroll Lock");
        assert_eq!(key(19), Some(KeyCode::Pause), "FreezeCamera = 19 is Pause/Break");
    }

    #[test]
    fn unknown_codes_are_none() {
        assert_eq!(key(0), None);
        assert_eq!(key(0x5B), None, "a Windows key is not bound by name");
    }
}
