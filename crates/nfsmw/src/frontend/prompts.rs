//! Contextual action hints, shared by every implemented front-end screen.

use blackbox_feng::{NodeKind, UiNode, UiTree, fe_hash_upper};
use glam::{Mat4, Quat, Vec3};

use super::ids::screen;
use crate::input::{Action, Bindings, InputDevice};
use crate::ui::{
    UiAssets,
    input_icons::{ATLAS, Glyph},
    prompts::{self, Prompt},
};

const FONT: u32 = 0x5455_70c6; // FONT_MW_BODY

pub fn decorate(tree: &mut UiTree, name: &str, bindings: &Bindings, device: InputDevice, assets: &UiAssets) {
    let (pause, rows, main, splash) = (
        name.eq_ignore_ascii_case(screen::PAUSE_MENU),
        [screen::OPTIONS, screen::PAUSE_OPTIONS].iter().any(|s| name.eq_ignore_ascii_case(s)),
        name.eq_ignore_ascii_case(screen::MAIN_MENU),
        [screen::SPLASH, screen::SPLASH_WIDE].iter().any(|s| name.eq_ignore_ascii_case(s)),
    );
    let supported = pause || rows || main || splash || name.eq_ignore_ascii_case(screen::MAIN_MENU_SUB);
    if !supported {
        return;
    }
    let splash_anchor = tree
        .nodes
        .iter()
        .find(|n| n.name_hash == 0xC4DF_3FF2)
        .map(|n| (n.world.transform_point3(Vec3::ZERO), n.world_colour[3], n.visible));
    hide_legacy_hints(tree, splash);
    let opacity = match (splash, splash_anchor) {
        (true, Some((_, alpha, true))) => alpha,
        (true, Some(_)) => 0,
        _ => 255,
    };
    if opacity == 0 {
        return;
    }
    let first_hint = tree.nodes.len();
    let action = |a| vec![prompts::for_action(bindings, a, device)];
    let mut hints = vec![(action(Action::MenuAccept), "Accept")];
    if splash {
        hints = vec![(action(Action::MenuStart), "Continue")];
    }
    if !splash {
        hints.push((action(Action::MenuBack), "Back"));
    }
    if main {
        hints[1] = (action(Action::MenuQuit), "Quit");
    }
    if rows {
        hints = vec![
            (prompts::pair(bindings, Action::MenuUp, Action::MenuDown, device), "Select"),
            (prompts::pair(bindings, Action::MenuLeft, Action::MenuRight, device), "Adjust"),
            (action(Action::MenuBack), "Done"),
        ];
    }
    let colour = match pause || name.eq_ignore_ascii_case(screen::PAUSE_OPTIONS) {
        true => [255, 174, 64, 255],
        false => [255; 4],
    };
    let (mut x, mut y, width) = match (pause, splash) {
        (true, _) => (-267.0, 67.0, 228.0),
        (_, true) => (-80.0, 140.0, 220.0),
        _ => (-280.0, 208.0, 560.0),
    };
    let total: f32 = hints.iter().map(|(keys, label)| hint_width(keys, label, assets)).sum();
    let fit = (width / total.max(1.0)).min(1.0);
    if splash && let Some((anchor, _, _)) = splash_anchor {
        x = anchor.x - total * fit * 0.5;
        y = anchor.y;
    }
    for (keys, label) in hints {
        for key in keys {
            let key_width = prompt_width(&key, assets) * fit;
            match key {
                Prompt::Icon(glyph) => image(tree, glyph, x + key_width * 0.5, y, 25.0 * fit),
                Prompt::Key(key) => {
                    let mut cap = node(
                        NodeKind::Image {
                            texture: fe_hash_upper("BASEPOLY"),
                            uv: [0.0, 0.0, 1.0, 1.0],
                            mask: None,
                            mask_rotation: [0.5, 0.5, 0.0],
                            mask_uv: [0.0, 0.0, 1.0, 1.0],
                        },
                        x + key_width * 0.5,
                        y,
                    );
                    cap.world *= Mat4::from_scale(Vec3::new(key_width - 3.0 * fit, 22.0 * fit, 1.0));
                    cap.world_colour = [25, 25, 25, 235];
                    push(tree, cap);
                    text(tree, &key, x + 4.0 * fit, y, 0.8 * fit, [255; 4]);
                }
            }
            x += key_width;
        }
        text(tree, label, x + 3.0 * fit, y, fit, colour);
        x += (text_width(label, assets) + 22.0) * fit;
    }
    for n in &mut tree.nodes[first_hint..] {
        n.world_colour[3] = (u16::from(n.world_colour[3]) * u16::from(opacity) / 255) as u8;
    }
}

fn prompt_width(prompt: &Prompt, assets: &UiAssets) -> f32 {
    match prompt {
        Prompt::Icon(_) => 27.0,
        Prompt::Key(key) => text_width(key, assets) * 0.8 + 12.0,
    }
}
fn hint_width(keys: &[Prompt], label: &str, assets: &UiAssets) -> f32 {
    keys.iter().map(|p| prompt_width(p, assets)).sum::<f32>() + text_width(label, assets) + 22.0
}
fn text_width(text: &str, assets: &UiAssets) -> f32 {
    use blackbox_feng::font::TextStyle;
    let Some(font) = assets.font(FONT) else { return text.len() as f32 * 8.0 };
    font.layout(
        text,
        TextStyle { justification: 0, leading: 0, max_width: 0 },
        assets.texture_size(font.texture_hash).unwrap_or((256, 256)),
    )
    .width
}

fn hide_legacy_hints(tree: &mut UiTree, splash: bool) {
    let names = [0x6A21_8478, 0x812A_09D4, super::ids::QUIT_BUTTON, 0xC4DF_3FF2];
    let mut hidden = vec![false; tree.nodes.len()];
    for (i, n) in tree.nodes.iter().enumerate() {
        let hint_text = n.text.as_deref().is_some_and(|t| matches!(t.trim(), "Accept" | "Back" | "Quit" | "Defaults"));
        hidden[i] = hint_text || names[..3].contains(&n.name_hash) || (splash && n.name_hash == names[3]);
    }
    // Ancestors precede descendants in the actual packages; the parent chain also handles other orderings.
    for i in 0..tree.nodes.len() {
        let mut parent = tree.nodes[i].parent;
        let mut budget = tree.nodes.len();
        while let Some(p) = parent.filter(|p| *p < tree.nodes.len()) {
            if hidden[p] {
                hidden[i] = true;
                break;
            }
            if budget == 0 {
                break;
            }
            budget -= 1;
            parent = tree.nodes[p].parent;
        }
    }
    tree.draw_order.retain(|i| !hidden[*i]);
}

fn node(kind: NodeKind, x: f32, y: f32) -> UiNode {
    let pos = Vec3::new(x, y, 1.0);
    UiNode {
        index: 0,
        guid: 0,
        name_hash: 0,
        parent: None,
        kind,
        text: None,
        local_position: pos,
        position: pos,
        local_pivot: Vec3::ZERO,
        local_rotation: Quat::IDENTITY,
        size: Vec3::ONE,
        colour: [255; 4],
        world_colour: [255; 4],
        world: Mat4::from_translation(pos),
        z: 1.0,
        visible: true,
        clip: None,
    }
}
fn push(tree: &mut UiTree, mut node: UiNode) {
    node.index = tree.nodes.len();
    tree.draw_order.push(node.index);
    tree.nodes.push(node);
}
fn image(tree: &mut UiTree, glyph: Glyph, x: f32, y: f32, size: f32) {
    let mut n = node(
        NodeKind::Image {
            texture: ATLAS,
            uv: glyph.uv(),
            mask: None,
            mask_rotation: [0.5, 0.5, 0.0],
            mask_uv: [0.0, 0.0, 1.0, 1.0],
        },
        x,
        y,
    );
    n.world *= Mat4::from_scale(Vec3::new(size, size, 1.0));
    push(tree, n);
}
fn text(tree: &mut UiTree, text: &str, x: f32, y: f32, scale: f32, colour: [u8; 4]) {
    let mut n = node(NodeKind::Text { font: FONT, justification: 4, leading: 0, max_width: 0 }, x, y);
    n.text = Some(text.into());
    n.world *= Mat4::from_scale(Vec3::new(scale, scale, 1.0));
    n.world_colour = colour;
    push(tree, n);
}
