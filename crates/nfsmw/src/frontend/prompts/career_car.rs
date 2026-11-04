//! Career car selection shares the package with Quick Race, but not its Showcase or sale actions.

use super::{Hint, HintLabel as L, legacy};
use crate::frontend::logic::Footer;
use crate::input::{Action, Bindings, InputDevice};
use crate::ui::prompts;

pub(super) fn hints(footer: Footer, bindings: &Bindings, device: InputDevice) -> Option<Vec<Hint>> {
    let Footer::CareerCarSelect { accept, marker, back } = footer else { return None };
    let mut hints = Vec::new();
    let action = |a| vec![prompts::for_action(bindings, a, device)];
    if back {
        hints.push((action(Action::MenuBack), L::Back, Some(Action::MenuBack)));
    }
    hints.push((prompts::pair(bindings, Action::MenuLeft, Action::MenuRight, device), L::Select, None));
    if let Some(label) = accept {
        hints.push((action(Action::MenuAccept), L::Game(label), Some(Action::MenuAccept)));
    }
    if marker {
        hints.push((action(Action::MenuButton2), L::Game(0xf9c7_3cc2), Some(Action::MenuButton2)));
    }
    Some(hints)
}

pub(super) fn hide_legacy(tree: &mut blackbox_feng::UiTree) {
    legacy::hide_children(tree, 0x84bc_f2b5);
    // Native PC action cards are rooted separately from the animated console footer.
    legacy::hide_roots(tree, &[0xc0a3_2823, 0x73dc_b662, 0x4b89_0cdb, 0xb66f_0fec, 0xa5e3_45ef, 0xd42f_677b]);
}
