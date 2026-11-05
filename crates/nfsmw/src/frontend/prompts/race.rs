//! Race result prompts, using the same input bindings and atlas as other menus.
use blackbox_feng::UiTree;
use glam::{Mat4, Vec3};

use super::{Hint, HintLabel as L, legacy};
use crate::frontend::{ids::screen, logic::Footer, prompt_layout::Frame};
use crate::input::{Action, Bindings, InputDevice};
use crate::ui::prompts;

const PHOTO_FOOTER: u32 = 0x37AC_A28C;
const PHOTO_CONTINUE: u32 = 0xA2D2_1D6F;
const RESULTS_FOOTER: u32 = 0xFF12_69D7;
const RESULTS_CONSOLE: u32 = 0x02DD_F58C;

/// Retain the photo artwork beneath another screen, with no second set of buttons.
pub(in crate::frontend) fn cover_photo(tree: &mut UiTree) {
    hide_legacy(tree, screen::PHOTO);
    legacy::hide_roots(tree, &[PHOTO_FOOTER, 0xE057_94EB, 0x895E_5289, 0xC591_84C1, 0xFBE6_5EB3]);
}

pub(super) fn hints(footer: Footer, bindings: &Bindings, device: InputDevice) -> Option<Vec<Hint>> {
    let hint = |action, label| (vec![prompts::for_action(bindings, action, device)], label, Some(action));
    let mut hints = vec![hint(Action::MenuAccept, L::Continue)];
    match footer {
        Footer::RacePhoto { race } => {
            if race {
                hints.push(hint(Action::MenuButton2, L::Game(0xD1F2_C488)));
                hints.push(hint(Action::MenuButton1, L::Game(0xF893_AFA1)));
            }
        }
        Footer::RaceResults { back, page } => {
            hints[0] = hint(Action::MenuAccept, L::Game(0xA995_0B93));
            if back {
                hints.push(hint(Action::MenuBack, L::Back));
            }
            hints.push(hint(Action::MenuButton2, L::Game(page)));
            hints.push(hint(Action::MenuButton1, L::Game(0xF893_AFA1)));
        }
        _ => return None,
    }
    Some(hints)
}

pub(in crate::frontend) fn frame(tree: &UiTree, name: &str) -> Option<Frame> {
    let root_hash = match name.to_ascii_lowercase().as_str() {
        "ingamephotomaster.fng" => PHOTO_FOOTER,
        "postrace_results.fng" => RESULTS_FOOTER,
        _ => return None,
    };
    let root = tree.nodes.iter().find(|node| node.name_hash == root_hash)?;
    if root_hash == PHOTO_FOOTER {
        let bar = tree.nodes.iter().find(|node| node.parent == Some(root.index) && node.name_hash == 0x81B4_FAD0)?;
        let world = Mat4::from_translation(Vec3::new(bar.world.w_axis.x, bar.world.w_axis.y, root.world.w_axis.z));
        // The authored backing reaches alpha 180. Its fade gates the replacement glyphs too.
        let fade = (u32::from(bar.world_colour[3]) * 255 / 180).min(255) as u8;
        return Some(Frame::new(root, world, bar.size.x.abs() - 12.0, bar.size.y.abs(), false).faded(fade));
    }
    let at = Vec3::new(0.0, root.world.w_axis.y, root.world.w_axis.z);
    Some(Frame::new(root, Mat4::from_translation(at), 548.0, 25.0, false))
}

pub(super) fn hide_legacy(tree: &mut UiTree, name: &str) {
    if name.eq_ignore_ascii_case(screen::PHOTO) {
        legacy::hide_roots(tree, &[0x5CED_1D04, 0x6A21_8478, 0xD646_3141, 0xC0AC_B81A]);
        legacy::hide_children(tree, PHOTO_CONTINUE);
    }
    if name.eq_ignore_ascii_case(screen::POST_RACE_RESULTS) {
        legacy::hide_children(tree, RESULTS_FOOTER);
        legacy::hide_roots(tree, &[RESULTS_CONSOLE]);
    }
}
