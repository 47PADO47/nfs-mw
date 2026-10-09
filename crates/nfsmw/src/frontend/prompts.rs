//! Contextual action hints, shared by every implemented front-end screen.

use blackbox_feng::{NodeKind, UiNode, UiTree, fe_hash_upper};
use glam::{Mat4, Quat, Vec3};

use super::{ids::screen, prompt_layout::Frame};
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
    let Some(frame) = Frame::find(tree, name) else { return };
    hide_legacy_hints(tree, splash);
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
    let total: f32 = hints.iter().map(|(keys, label)| hint_width(keys, label, assets)).sum();
    let fit = (frame.width / total.max(1.0)).min((frame.height - 2.0) / 25.0).min(1.0);
    let mut x = match frame.centred {
        true => -total * fit * 0.5,
        false => -frame.width * 0.5,
    };
    for (keys, label) in hints {
        for key in keys {
            let key_width = prompt_width(&key, assets) * fit;
            match key {
                Prompt::Icon(glyph) => image(tree, glyph, x + key_width * 0.5, 0.0, 25.0 * fit),
                Prompt::Key(key) => keycap(tree, &key, x, key_width, fit, colour, assets),
            }
            x += key_width;
        }
        text(tree, label, x + 3.0 * fit, 0.0, fit, colour, assets);
        x += (text_width(label, assets) + 22.0) * fit;
    }
    frame.apply(tree, first_hint);
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

fn keycap(tree: &mut UiTree, key: &str, x: f32, width: f32, fit: f32, colour: [u8; 4], assets: &UiAssets) {
    let (cx, w, h, edge) = (x + width * 0.5, width - 3.0 * fit, 22.0 * fit, 1.3 * fit);
    // Open corners keep the border light; a translucent inset preserves the menu underneath.
    rect(tree, cx, 0.0, w - 2.0 * edge, h - 2.0 * edge, [0, 0, 0, 150]);
    for y in [-h * 0.5 + edge * 0.5, h * 0.5 - edge * 0.5] {
        rect(tree, cx, y, w - 2.0 * edge, edge, colour);
    }
    for dx in [-w * 0.5 + edge * 0.5, w * 0.5 - edge * 0.5] {
        rect(tree, cx + dx, 0.0, edge, h - 2.0 * edge, colour);
    }
    text(tree, key, cx - text_width(key, assets) * 0.4 * fit, 0.0, 0.8 * fit, colour, assets);
}

fn rect(tree: &mut UiTree, x: f32, y: f32, width: f32, height: f32, colour: [u8; 4]) {
    let mut n = node(
        NodeKind::Image {
            texture: fe_hash_upper("BASEPOLY"),
            uv: [0.0, 0.0, 1.0, 1.0],
            mask: None,
            mask_rotation: [0.5, 0.5, 0.0],
            mask_uv: [0.0, 0.0, 1.0, 1.0],
        },
        x,
        y,
    );
    n.world *= Mat4::from_scale(Vec3::new(width, height, 1.0));
    n.world_colour = colour;
    push(tree, n);
}

fn hide_legacy_hints(tree: &mut UiTree, splash: bool) {
    let names = [0x6A21_8478, 0x812A_09D4, 0x7379_349B, 0x5E4D_1BDC, super::ids::QUIT_BUTTON];
    let mut hidden = vec![false; tree.nodes.len()];
    for (i, n) in tree.nodes.iter().enumerate() {
        let hint_text = n.text.as_deref().is_some_and(|t| matches!(t.trim(), "Accept" | "Back" | "Quit" | "Defaults"));
        hidden[i] = hint_text || names.contains(&n.name_hash) || (splash && n.name_hash == 0xC4DF_3FF2);
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
fn text(tree: &mut UiTree, text: &str, x: f32, y: f32, scale: f32, colour: [u8; 4], assets: &UiAssets) {
    use blackbox_feng::font::TextStyle;
    let offset = assets.font(FONT).map_or(0.0, |font| {
        let layout = font.layout(
            text,
            TextStyle { justification: 4, leading: 0, max_width: 0 },
            assets.texture_size(font.texture_hash).unwrap_or((256, 256)),
        );
        let top = layout.quads.iter().map(|q| q.y0).reduce(f32::min).unwrap_or(0.0);
        let bottom = layout.quads.iter().map(|q| q.y1).reduce(f32::max).unwrap_or(0.0);
        (top + bottom) * 0.5
    });
    let mut n = node(NodeKind::Text { font: FONT, justification: 4, leading: 0, max_width: 0 }, x, y);
    n.text = Some(text.into());
    n.world *= Mat4::from_scale(Vec3::new(scale, scale, 1.0)) * Mat4::from_translation(Vec3::new(0.0, -offset, 0.0));
    n.world_colour = colour;
    push(tree, n);
}
