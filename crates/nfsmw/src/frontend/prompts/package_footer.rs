//! Screens that draw their package's own footer instead of the host's hint row (`docs/specs/fe-package-footer.md`).
//!
//! Their packages carry two copies of every footer button: the keyboard's (object flag [`KEYBOARD`], `PC_key_*` art,
//! started hidden and shown by the package's own scripts) and the pad's (object flag [`PAD`], the console button art). The
//! original hides every pad object of a package when the package starts and does not show one again while the keyboard
//! is in use; while a pad is in use it shows the pad copies instead, with the button art of the pad. Here the copy of
//! the other device is taken out of the presented tree, and on a pad each button's icon is the remake's glyph of the
//! action it stands for. State the package cannot know (a speed trap has no Race Stats, a plain message no Re-Play)
//! hides whole buttons.

use blackbox_feng::package::{ObjectDef, ObjectKind};
use blackbox_feng::{NodeKind, ObjectRef, PackageId, Runtime, UiNode, UiTree};
use glam::{Vec2, Vec3};

use super::{HintArea, legacy};
use crate::frontend::{ids::screen, logic::Footer};
use crate::i18n;
use crate::input::{Action, Bindings, InputDevice};
use crate::ui::input_icons::ATLAS;
use crate::ui::prompts::{self, Prompt};

/// The object flag of the keyboard's copy of a hint (the runtime also starts these hidden).
pub const KEYBOARD: u32 = 1 << 3;
/// The object flag of the pad's copy of a hint.
pub const PAD: u32 = 1 << 6;

/// The `PC_key_*` pictures: a keyboard group holding one is a footer hint (other start-hidden objects are panels).
const KEY_ART: [u32; 5] = [0x1BB3_C514, 0x68A2_9A8A, 0x68A2_9A8B, 0x68A2_9A8C, 0xD7B6_EDB7];

/// One footer button of a package: its keyboard and pad groups, and the action it presses.
struct Button {
    keys: u32,
    pad: u32,
    action: Action,
}

const fn button(keys: u32, pad: u32, action: Action) -> Button {
    Button { keys, pad, action }
}

/// The photo's buttons: Continue, Race Stats, Restart.
const PHOTO: [Button; 3] = [
    button(0x6CD6_A337, 0x812A_09D4, Action::MenuAccept),
    button(0xD646_3141, 0x5CED_1D04, Action::MenuButton2),
    button(0xC0AC_B81A, 0x6A21_8478, Action::MenuButton1),
];
/// The results' buttons: Exit, the page, Restart.
const RESULTS: [Button; 3] = [
    button(0x9502_D148, 0xB36C_C7CB, Action::MenuAccept),
    button(0x3FC7_2F9D, 0x5CED_1D04, Action::MenuButton2),
    button(0xB9A5_851F, 0x6A21_8478, Action::MenuButton1),
];
/// The results' online buttons (Gamer Card, Feedback): pad groups the original never shows offline.
const RESULTS_ONLINE: [u32; 2] = [0xD9CF_B4EB, 0xCD06_6337];
/// The results' page caption on the keyboard's button (the results logic writes it) and the pad copy's two strings
/// (caption and shadow), which take the same text.
const RESULTS_PAGE: u32 = 0x6F23_15C4;
const RESULTS_PAGE_PAD: [u32; 2] = [0x5CED_1D04, 0x6E26_3D09];
/// The message's buttons: Done, Delete, Re-Play.
const MESSAGE: [Button; 3] = [
    button(0x5232_50E9, 0x1C7F_CF8B, Action::MenuBack),
    button(0xC1C1_90EB, 0xD18B_818D, Action::MenuButton1),
    button(0x9146_14E5, 0x2A63_1207, Action::MenuButton2),
];
/// The inbox's buttons: Read, Done, Delete.
const INBOX: [Button; 3] = [
    button(0x9502_D148, 0x23AF_B3F6, Action::MenuAccept),
    button(0x5232_50E9, 0x1C7F_CF8B, Action::MenuBack),
    button(0xC1C1_90EB, 0xD18B_818D, Action::MenuButton1),
];
/// The car select's buttons (the car lot's, the quick race's and the career's): Back, Accept, Showcase, and the Sell and
/// Load Cars that no selection here offers.
const CAR_SELECT: [Button; 3] = [
    button(0xC0A3_2823, 0x02DD_F58C, Action::MenuBack),
    button(0x73DC_B662, 0x7379_349B, Action::MenuAccept),
    button(0x18A4_384F, 0x4B89_0CDB, Action::MenuButton3),
];
const CAR_SELECT_UNUSED: [u32; 3] = [0xD646_3100, 0xD42F_677B, 0x5F34_A28C];
/// The captions of the car select's Accept (keyboard, pad).
const CAR_SELECT_ACCEPT: [u32; 2] = [0x0013_35F0, 0xB941_39F4];
const BIO_CONTINUE: [Button; 1] = [button(0xDDE3_DB34, 0x1E61_7016, Action::MenuAccept)];

/// The buttons of a screen that draws its package's footer, or `None` for a screen with the host's hint row.
fn buttons(name: &str, footer: Footer) -> Option<&'static [Button]> {
    const LOADING_TIP: [Button; 1] = [button(0xB857_7932, 0x381F_EA80, Action::MenuAccept)];
    let is = |file: &str| name.eq_ignore_ascii_case(file);
    if is(screen::LOADING_TIPS) {
        return Some(&LOADING_TIP);
    }
    match footer {
        Footer::RivalBioContinue if is(screen::SAFE_HOUSE_RIVAL_BIO) => Some(&BIO_CONTINUE),
        Footer::RacePhoto { .. } if is(screen::PHOTO) => Some(&PHOTO),
        Footer::RaceResults { .. } if is(screen::POST_RACE_RESULTS) => Some(&RESULTS),
        Footer::SmsMessage { .. } => Some(&MESSAGE),
        Footer::SmsInbox { .. } => Some(&INBOX),
        Footer::CarSelect | Footer::CarSelectLocked if is(screen::CAR_SELECT) => Some(&CAR_SELECT),
        // The strike-marker offer has no keyboard button in the package: that state keeps the host's hint row.
        Footer::CareerCarSelect { marker: false, .. } if is(screen::CAR_SELECT) => Some(&CAR_SELECT),
        _ => None,
    }
}

/// Whether the screen draws its package's own footer.
pub fn owns(name: &str, footer: Footer) -> bool {
    buttons(name, footer).is_some()
}

/// Shows the copy of the footer for `device`, hides what the screen's state rules out, and returns the buttons a click
/// presses. `flags` gives a node's package flags (0 for nodes that are no object of the package).
pub fn apply(
    tree: &mut UiTree,
    flags: impl Fn(&UiNode) -> u32,
    name: &str,
    footer: Footer,
    bindings: &Bindings,
    device: InputDevice,
) -> Vec<HintArea> {
    let Some(buttons) = buttons(name, footer) else { return Vec::new() };
    let pad = device != InputDevice::Keyboard;
    let flags: Vec<u32> = tree.nodes.iter().map(&flags).collect();
    let other: Vec<usize> = match pad {
        false => (0..tree.nodes.len()).filter(|&i| flags[i] & PAD != 0).collect(),
        true => (0..tree.nodes.len()).filter(|&i| flags[i] & KEYBOARD != 0 && holds_key_art(tree, i)).collect(),
    };
    hide_subtrees(tree, &other);
    hide_by_state(tree, footer);
    if footer == Footer::RivalBioContinue {
        let [continue_button] = BIO_CONTINUE;
        let other = if pad { continue_button.keys } else { continue_button.pad };
        legacy::hide_roots(tree, &[other]);
    }
    if pad && footer == (Footer::RacePhoto { race: true }) {
        // On the pad the offset Race Stats shadow and the caption have identical depth in the PC package.
        let position = |guid| tree.draw_order.iter().position(|&i| tree.nodes[i].guid == guid);
        if let (Some(caption), Some(shadow)) = (position(0x000F_C866), position(0x000F_C868))
            && caption < shadow
        {
            tree.draw_order.swap(caption, shadow);
        }
    }
    if pad {
        for b in buttons {
            set_pad_icons(tree, b.pad, prompts::for_action(bindings, b.action, device));
        }
    }
    buttons
        .iter()
        .filter_map(|b| {
            let group = match pad {
                true => b.pad,
                false => b.keys,
            };
            area(tree, group).map(|(min, max)| HintArea { action: b.action, min, max })
        })
        .collect()
}

/// The buttons the screen's state rules out, and the captions it names.
fn hide_by_state(tree: &mut UiTree, footer: Footer) {
    match footer {
        Footer::RacePhoto { race: false } => {
            legacy::hide_roots(tree, &[PHOTO[1].keys, PHOTO[1].pad, PHOTO[2].keys, PHOTO[2].pad]);
        }
        Footer::RaceResults { .. } => {
            legacy::hide_roots(tree, &RESULTS_ONLINE);
            let caption = tree.nodes.iter().find(|n| n.name_hash == RESULTS_PAGE).and_then(|n| n.text.clone());
            if let Some(text) = caption {
                set_text(tree, &RESULTS_PAGE_PAD, &text);
            }
        }
        // The original's message setup hides the Re-Play buttons of a message that is not a voice message.
        Footer::SmsMessage { replay: false } => legacy::hide_roots(tree, &[MESSAGE[2].keys, MESSAGE[2].pad]),
        Footer::CarSelect => legacy::hide_roots(tree, &CAR_SELECT_UNUSED),
        // A locked car: Back only.
        Footer::CarSelectLocked => {
            let mut hidden = CAR_SELECT_UNUSED.to_vec();
            hidden.extend([CAR_SELECT[1].keys, CAR_SELECT[1].pad, CAR_SELECT[2].keys, CAR_SELECT[2].pad]);
            legacy::hide_roots(tree, &hidden);
        }
        Footer::CareerCarSelect { accept, back, .. } => {
            let mut hidden = CAR_SELECT_UNUSED.to_vec();
            hidden.extend([CAR_SELECT[2].keys, CAR_SELECT[2].pad]);
            if !back {
                hidden.extend([CAR_SELECT[0].keys, CAR_SELECT[0].pad]);
            }
            match accept.and_then(i18n::game) {
                Some(text) => {
                    let captions: Vec<usize> = [CAR_SELECT[1].keys, CAR_SELECT[1].pad]
                        .iter()
                        .flat_map(|&group| descendants(tree, group))
                        .filter(|&i| CAR_SELECT_ACCEPT.contains(&tree.nodes[i].name_hash))
                        .collect();
                    for i in captions {
                        tree.nodes[i].text = Some(text.clone());
                    }
                }
                None if accept.is_none() => hidden.extend([CAR_SELECT[1].keys, CAR_SELECT[1].pad]),
                None => {}
            }
            legacy::hide_roots(tree, &hidden);
        }
        _ => {}
    }
}

/// The packages whose footer the remake draws as the package's own (for [`reveal_keys`], before their logic starts).
const PACKAGES: [&str; 5] =
    [screen::PHOTO, screen::POST_RACE_RESULTS, screen::SMS_MESSAGE, screen::SMS_MAILBOXES, screen::CAR_SELECT];

/// Shows the keyboard objects of one of these packages as it loads, before its logic starts (whose own hides win). The
/// runtime starts objects with the keyboard flag hidden until the package's own script reveals them; the original draws
/// them from the start (read live with the keyboard on the photo, the results and the car lot), and the presented tree
/// drops the copy of the device not in use (`apply`).
pub fn reveal_keys(rt: &mut Runtime, package: PackageId, name: &str) {
    if !PACKAGES.iter().any(|file| name.eq_ignore_ascii_case(file)) {
        return;
    }
    let Some(def) = rt.package(package) else { return };
    // An object other than a group with its own invisible bit (bit 0) set is not drawn by the original either; a group's
    // bit is not tested (its children are drawn).
    let shown = |o: &ObjectDef| o.flags & KEYBOARD != 0 && (o.kind == ObjectKind::Group || o.flags & 1 == 0);
    let keys: Vec<usize> = (0..def.objects.len()).filter(|&i| shown(&def.objects[i])).collect();
    for index in keys {
        rt.set_hidden(ObjectRef { package, index }, false);
    }
}

/// Whether a node is, or holds, a `PC_key_*` picture.
fn holds_key_art(tree: &UiTree, root: usize) -> bool {
    let is_key = |n: &UiNode| matches!(n.kind, NodeKind::Image { texture, .. } if KEY_ART.contains(&texture));
    is_key(&tree.nodes[root]) || descendants_of(tree, root).into_iter().any(|i| is_key(&tree.nodes[i]))
}

/// The nodes below the first node named `group` (not the group itself).
fn descendants(tree: &UiTree, group: u32) -> Vec<usize> {
    let Some(root) = tree.nodes.iter().position(|n| n.name_hash == group) else { return Vec::new() };
    descendants_of(tree, root)
}

fn descendants_of(tree: &UiTree, root: usize) -> Vec<usize> {
    (0..tree.nodes.len()).filter(|&i| i != root && has_ancestor(tree, i, root)).collect()
}

fn has_ancestor(tree: &UiTree, node: usize, ancestor: usize) -> bool {
    let mut at = tree.nodes[node].parent;
    for _ in 0..tree.nodes.len() {
        let Some(p) = at.filter(|p| *p < tree.nodes.len()) else { return false };
        if p == ancestor {
            return true;
        }
        at = tree.nodes[p].parent;
    }
    false
}

/// Takes the nodes and everything below them out of the presented tree.
fn hide_subtrees(tree: &mut UiTree, roots: &[usize]) {
    if roots.is_empty() {
        return;
    }
    let hidden: Vec<bool> =
        (0..tree.nodes.len()).map(|i| roots.contains(&i) || roots.iter().any(|&r| has_ancestor(tree, i, r))).collect();
    for (node, hide) in tree.nodes.iter_mut().zip(&hidden) {
        node.visible &= !hide;
    }
    tree.draw_order.retain(|i| !hidden[*i]);
}

fn set_text(tree: &mut UiTree, names: &[u32], text: &str) {
    for node in tree.nodes.iter_mut().filter(|n| names.contains(&n.name_hash) && n.text.is_some()) {
        node.text = Some(text.to_owned());
    }
}

/// Draws the remake's glyph of the pad button on every picture of a pad group. A binding with no glyph (a key) keeps
/// the package's picture.
fn set_pad_icons(tree: &mut UiTree, group: u32, prompt: Prompt) {
    let Prompt::Icon(glyph) = prompt else { return };
    for i in descendants(tree, group) {
        if let NodeKind::Image { texture, uv, .. } = &mut tree.nodes[i].kind {
            *texture = ATLAS;
            *uv = glyph.uv();
        }
    }
}

/// The rectangle a group's drawn pictures cover on the screen, in FEng units.
fn area(tree: &UiTree, group: u32) -> Option<(Vec2, Vec2)> {
    let drawn: Vec<usize> = descendants(tree, group)
        .into_iter()
        .filter(|i| tree.nodes[*i].visible && tree.draw_order.contains(i))
        .collect();
    let mut corners = drawn.iter().filter(|&&i| matches!(tree.nodes[i].kind, NodeKind::Image { .. })).flat_map(|&i| {
        let world = tree.nodes[i].world;
        [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)]
            .map(|(x, y)| world.transform_point3(Vec3::new(x, y, 0.0)).truncate())
    });
    let first = corners.next()?;
    Some(corners.fold((first, first), |(min, max), p| (min.min(p), max.max(p))))
}

#[cfg(test)]
#[path = "package_footer_tests.rs"]
mod tests;
