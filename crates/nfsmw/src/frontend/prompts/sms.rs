//! The SMS inbox and message screens (`SMS_MailBoxes.fng`, `SMS_Message.fng`) carry two copies of their footer: the
//! keyboard's (`PC_key_*` art, start-hidden groups that the screens' intro scripts show) and the console's (pad button
//! art). Both stand on the same line, so the drawn hints replace both and follow the device that was used last.
//!
//! The order and the captions are the packages' own (`nfsmw dump-screen`): the list says Back, Read and Delete (keys Esc,
//! Enter and 1), the message says Done, Delete and Re-Play (Esc, 1 and 2). Key 1 sends `PAD_BUTTON5` and key 2
//! `PAD_BUTTON4`, which are the actions `MenuButton1` and `MenuButton2`.

use blackbox_feng::UiTree;
use glam::{Mat4, Vec3};

use super::{Hint, HintLabel as L, legacy};
use crate::frontend::logic::Footer;
use crate::frontend::prompt_layout::Frame;
use crate::input::{Action, Bindings, InputDevice};
use crate::ui::prompts;

/// The captions of the packages' hints: Read, Delete and Re-Play (`SMS_READ`, `DELETE`, `SMS_REPLAY`).
const READ: u32 = 0x61DE_6DA2;
const DELETE: u32 = 0x56B0_0632;
const REPLAY: u32 = 0xC207_AEFE;

/// The inbox's footer group: its bar and stripe stay, its console hints go.
const INBOX_FOOTER: u32 = 0xB3A6_9E89;
/// The inbox's keyboard hints: Read, Back and Delete.
const INBOX_KEYS: [u32; 3] = [0x9502_D148, 0x5232_50E9, 0xC1C1_90EB];
/// The message's console hints (one group) and keyboard hints (Re-Play, Done, Delete).
const MESSAGE_PAD: u32 = 0xB04B_66C8;
const MESSAGE_KEYS: [u32; 3] = [0x9146_14E5, 0x5232_50E9, 0xC1C1_90EB];
/// The keyboard's Done group of the message, whose fade the drawn hints follow.
const MESSAGE_DONE: u32 = 0x5232_50E9;
/// The message screen has no footer bar: the hints take the inbox's bar (560 wide, centred, 25 high, less its margin)
/// on the line of the package's own hints.
const MESSAGE_WIDTH: f32 = 548.0;
const MESSAGE_HEIGHT: f32 = 25.0;

pub(super) fn hints(footer: Footer, bindings: &Bindings, device: InputDevice) -> Option<Vec<Hint>> {
    let action = |a| vec![prompts::for_action(bindings, a, device)];
    let back = |label| (action(Action::MenuBack), label, Some(Action::MenuBack));
    let delete = (action(Action::MenuButton1), L::Game(DELETE), Some(Action::MenuButton1));
    match footer {
        Footer::SmsInbox { reading: true, .. } => Some(Vec::new()),
        Footer::SmsInbox { has_messages, .. } => {
            let mut hints = vec![back(L::Back)];
            if has_messages {
                hints.push((action(Action::MenuAccept), L::Game(READ), Some(Action::MenuAccept)));
                hints.push(delete);
            }
            Some(hints)
        }
        Footer::SmsMessage { replay } => {
            let mut hints = vec![back(L::Done), delete];
            if replay {
                hints.push((action(Action::MenuButton2), L::Game(REPLAY), Some(Action::MenuButton2)));
            }
            Some(hints)
        }
        _ => None,
    }
}

/// Hides both copies of the packages' own hints.
pub(super) fn hide_legacy(tree: &mut UiTree, footer: Footer) {
    match footer {
        Footer::SmsInbox { .. } => {
            legacy::hide_children(tree, INBOX_FOOTER);
            legacy::hide_roots(tree, &INBOX_KEYS);
        }
        Footer::SmsMessage { .. } => {
            legacy::hide_roots(tree, &[MESSAGE_PAD]);
            legacy::hide_roots(tree, &MESSAGE_KEYS);
        }
        _ => {}
    }
}

/// Where the message screen's hints go: the line of its keyboard hints, at their depth, following the Done group's
/// fade. The inbox has a bar and takes the common rule (`prompt_layout`).
pub(in crate::frontend) fn frame(tree: &UiTree, footer: Footer) -> Option<Frame> {
    let Footer::SmsMessage { .. } = footer else { return None };
    let root = tree.nodes.iter().find(|n| n.name_hash == MESSAGE_DONE)?;
    let at = Vec3::new(0.0, root.world.w_axis.y, root.world.w_axis.z);
    Some(Frame::new(root, Mat4::from_translation(at), MESSAGE_WIDTH, MESSAGE_HEIGHT, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn captions(footer: Footer, device: InputDevice) -> Vec<(L, Option<Action>)> {
        hints(footer, &Bindings::default(), device)
            .unwrap()
            .into_iter()
            .map(|(_, label, action)| (label, action))
            .collect()
    }

    #[test]
    fn the_inbox_offers_back_read_and_delete_in_the_packages_order() {
        let shown = captions(Footer::SmsInbox { has_messages: true, reading: false }, InputDevice::Keyboard);
        assert_eq!(
            shown,
            [
                (L::Back, Some(Action::MenuBack)),
                (L::Game(READ), Some(Action::MenuAccept)),
                (L::Game(DELETE), Some(Action::MenuButton1)),
            ]
        );
        let empty = captions(Footer::SmsInbox { has_messages: false, reading: false }, InputDevice::Keyboard);
        assert_eq!(empty, [(L::Back, Some(Action::MenuBack))], "an empty tab has nothing to read or delete");
        let under = captions(Footer::SmsInbox { has_messages: true, reading: true }, InputDevice::Keyboard);
        assert!(under.is_empty(), "the message's footer is the only one");
    }

    #[test]
    fn a_message_offers_replay_only_for_a_story_message() {
        let story = captions(Footer::SmsMessage { replay: true }, InputDevice::Keyboard);
        assert_eq!(
            story,
            [
                (L::Done, Some(Action::MenuBack)),
                (L::Game(DELETE), Some(Action::MenuButton1)),
                (L::Game(REPLAY), Some(Action::MenuButton2)),
            ]
        );
        assert_eq!(captions(Footer::SmsMessage { replay: false }, InputDevice::Keyboard).len(), 2);
    }

    #[test]
    fn the_keys_follow_the_device_used_last() {
        let footer = Footer::SmsMessage { replay: true };
        let keys = |device| -> Vec<_> {
            hints(footer, &Bindings::default(), device).unwrap().into_iter().map(|(keys, _, _)| keys).collect()
        };
        assert_ne!(keys(InputDevice::Keyboard), keys(InputDevice::Xbox), "the controller's buttons replace the keys");
    }

    #[test]
    fn other_footers_are_not_the_inboxes() {
        assert!(hints(Footer::Standard, &Bindings::default(), InputDevice::Keyboard).is_none());
    }
}
