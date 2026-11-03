//! Contextual action hints, shared by every implemented front-end screen.

use blackbox_feng::{NodeKind, UiNode, UiTree, fe_hash_upper};
use glam::{Mat4, Quat, Vec2, Vec3};

use super::logic::Footer;
use super::{ids::screen, prompt_layout::Frame};
use crate::career::CareerHandle;
use crate::input::{Action, Bindings, InputDevice};
use crate::ui::{
    UiAssets,
    input_icons::{ATLAS, Glyph},
    prompts::{self, Prompt},
};

mod legacy;

const FONT: u32 = 0x5455_70c6; // FONT_MW_BODY
/// Half the height of a hint in its frame's units, before the fit: the keycaps are 22 high.
const HINT_HALF_HEIGHT: f32 = 13.0;

/// A footer hint that a click presses: the action it stands for, and the rectangle it covers in FEng units.
pub struct HintArea {
    pub action: Action,
    pub min: Vec2,
    pub max: Vec2,
}

impl HintArea {
    pub fn covers(&self, point: Vec2) -> bool {
        (self.min.x..=self.max.x).contains(&point.x) && (self.min.y..=self.max.y).contains(&point.y)
    }
}

/// One footer hint: its keys, its label, and the action a click on it presses (none for a pair of arrows).
pub type Hint = (Vec<Prompt>, HintLabel, Option<Action>);

mod labels;
pub use labels::HintLabel;

/// `CSHOP_CLEAR`, the cart's Clear All: the garage's own footer is told apart by it (`garage_ui::prompts::hide_legacy`).
pub(in crate::frontend) const CLEAR_ALL: u32 = 0xA1C1_EEE2;

/// The game's hint labels that the packages' own footers show, which the drawn hints replace.
const LEGACY_HINTS: [u32; 9] = [
    0x4F87_6B6F, // ACCEPT
    0x159D_2F37, // COMMON_ACCEPT
    0x2B65_A216, // H_ACCEPT
    0x0013_35F0, // BACK
    0x8CD5_67B8, // COMMON_BACK
    0x61D5_96D7, // H_BACK
    0xC51F_BF74, // PC_QUIT
    0x2249_36F4, // OLX_QUIT
    0x00CA_369E, // H_DEFAULTS
];

/// The footer hints a screen shows, in order. The Original footer and the Remastered menus' draw the same set.
pub fn hint_set(name: &str, footer: Footer, bindings: &Bindings, device: InputDevice) -> Vec<Hint> {
    use HintLabel as L;
    if let Some(hints) = super::garage_ui::prompts::hints(footer, bindings, device) {
        return hints;
    }
    let (rows, main, splash) = (
        [screen::OPTIONS, screen::PAUSE_OPTIONS].iter().any(|s| name.eq_ignore_ascii_case(s)),
        name.eq_ignore_ascii_case(screen::MAIN_MENU),
        [screen::SPLASH, screen::SPLASH_WIDE].iter().any(|s| name.eq_ignore_ascii_case(s)),
    );
    let action = |a| vec![prompts::for_action(bindings, a, device)];
    let (back, accept) = (
        (action(Action::MenuBack), L::Back, Some(Action::MenuBack)),
        (action(Action::MenuAccept), L::Accept, Some(Action::MenuAccept)),
    );
    // The custom race's screens list Back before Accept, as the reference screenshots do.
    if [screen::SAFE_HOUSE_RACE_SHEET, "SafeHouseMilestones.fng", "SafeHouseBounty.fng"]
        .iter()
        .any(|s| name.eq_ignore_ascii_case(s))
    {
        let mut hints =
            vec![back, (prompts::pair(bindings, Action::MenuUp, Action::MenuDown, device), L::Select, None)];
        if footer == Footer::BackAccept {
            hints.push(accept);
        }
        return hints;
    }
    if name.eq_ignore_ascii_case(screen::SAFE_HOUSE_RIVAL_BIO) {
        return vec![(action(Action::MenuAccept), L::Continue, Some(Action::MenuAccept))];
    }
    match footer {
        Footer::ControlsCapture => {
            return vec![(
                vec![Prompt::Key(prompts::key_word(bevy_input::keyboard::KeyCode::Escape))],
                L::Cancel,
                None,
            )];
        }
        Footer::ControlsPages => {
            return vec![
                back,
                (prompts::pair(bindings, Action::MenuUp, Action::MenuDown, device), L::Select, None),
                (prompts::pair(bindings, Action::MenuLeft, Action::MenuRight, device), L::Column, None),
                (action(Action::MenuAccept), L::Remap, Some(Action::MenuAccept)),
                (prompts::pair(bindings, Action::MenuTriggerLeft, Action::MenuTriggerRight, device), L::Device, None),
            ];
        }
        Footer::Jukebox { reorder, has_songs } => {
            let mut hints = vec![back, accept];
            if has_songs {
                let label = match reorder {
                    true => L::Move,
                    false => L::Select,
                };
                hints.push((prompts::pair(bindings, Action::MenuUp, Action::MenuDown, device), label, None));
                if !reorder {
                    hints.push((
                        prompts::pair(bindings, Action::MenuLeft, Action::MenuRight, device),
                        L::ChangeMode,
                        None,
                    ));
                }
                hints.push((action(Action::JukeboxGrab), L::Grab, Some(Action::JukeboxGrab)));
                hints.push((action(Action::MenuButton2), L::Preview, Some(Action::MenuButton2)));
            }
            hints.push((
                prompts::pair(bindings, Action::MenuTriggerLeft, Action::MenuTriggerRight, device),
                L::Order,
                None,
            ));
            return hints;
        }
        Footer::BackOnly | Footer::CarSelectLocked => return vec![back],
        Footer::BackAccept => return vec![back, accept],
        Footer::Tutorial => {
            return vec![back, accept, (action(Action::MenuButton2), L::ViewTutorial, Some(Action::MenuButton2))];
        }
        Footer::Defaults => {
            return vec![back, accept, (action(Action::MenuButton1), L::Defaults, Some(Action::MenuButton1))];
        }
        Footer::CarSelect => {
            return vec![back, accept, (action(Action::MenuButton3), L::Showcase, Some(Action::MenuButton3))];
        }
        Footer::Orbit => {
            let zoom: Vec<Prompt> = [Action::MenuZoomIn, Action::MenuZoomOut]
                .into_iter()
                .map(|a| prompts::for_action(bindings, a, device))
                .collect();
            let arrows: Vec<Prompt> = [Action::MenuUp, Action::MenuDown, Action::MenuLeft, Action::MenuRight]
                .into_iter()
                .map(|a| prompts::for_action(bindings, a, device))
                .collect();
            return vec![back, (zoom, L::Zoom, None), (arrows, L::ViewCar, None)];
        }
        Footer::Standard
        | Footer::GarageManager { .. }
        | Footer::GarageMain { .. }
        | Footer::GarageCategories { .. }
        | Footer::GarageParts
        | Footer::GarageSelect
        | Footer::GarageCart { .. }
        | Footer::GarageTuning
        | Footer::GarageTuningHelp
        | Footer::GarageNumbers => {}
    }
    // Each hint names the action a click on it presses; the pairs of arrows have none.
    let mut hints = vec![(action(Action::MenuAccept), L::Accept, Some(Action::MenuAccept))];
    if splash {
        hints = vec![(action(Action::MenuStart), L::Continue, Some(Action::MenuStart))];
    }
    if !splash {
        hints.push((action(Action::MenuBack), L::Back, Some(Action::MenuBack)));
    }
    if main {
        // Quit, and Game Stats while a profile is loaded (the reference screenshot has no Accept here).
        hints = vec![(action(Action::MenuQuit), L::Quit, Some(Action::MenuQuit))];
        if CareerHandle::shared().is_loaded() {
            hints.push((action(Action::MenuButton1), L::GameStats, Some(Action::MenuButton1)));
        }
    }
    let is = |file: &str| name.eq_ignore_ascii_case(file);
    if is(screen::CONTROLS) || is(screen::PAUSE_CONTROLS) {
        hints = vec![
            (prompts::pair(bindings, Action::MenuUp, Action::MenuDown, device), L::Select, None),
            (prompts::pair(bindings, Action::MenuLeft, Action::MenuRight, device), L::Column, None),
            (action(Action::MenuAccept), L::Remap, Some(Action::MenuAccept)),
            (action(Action::MenuBack), L::Back, Some(Action::MenuBack)),
        ];
    }
    if is(screen::SAFE_HOUSE_RIVAL_CHALLENGE) {
        hints = vec![
            (prompts::pair(bindings, Action::MenuUp, Action::MenuDown, device), L::Select, None),
            (action(Action::MenuAccept), L::ViewRival, Some(Action::MenuAccept)),
            (action(Action::MenuBack), L::Back, Some(Action::MenuBack)),
        ];
    }
    if is(screen::SAFE_HOUSE_RIVAL_BIO) {
        hints = vec![(action(Action::MenuBack), L::Back, Some(Action::MenuBack))];
    }
    if is(screen::CREDITS) {
        hints = vec![(action(Action::MenuBack), L::Back, Some(Action::MenuBack))];
    }
    if is(screen::QUICK_RACE_BRIEF) {
        hints = vec![
            (action(Action::MenuBack), L::Back, Some(Action::MenuBack)),
            (action(Action::MenuAccept), L::Accept, Some(Action::MenuAccept)),
            (action(Action::MenuButton1), L::Randomize, Some(Action::MenuButton1)),
        ];
    }
    if rows {
        hints = vec![
            (prompts::pair(bindings, Action::MenuUp, Action::MenuDown, device), L::Select, None),
            (prompts::pair(bindings, Action::MenuLeft, Action::MenuRight, device), L::Adjust, None),
            (action(Action::MenuBack), L::Done, Some(Action::MenuBack)),
        ];
    }
    hints
}

/// Draws the footer hints of a screen and returns the ones that a click can press.
pub fn decorate(
    tree: &mut UiTree,
    name: &str,
    footer: Footer,
    bindings: &Bindings,
    device: InputDevice,
    assets: &UiAssets,
) -> Vec<HintArea> {
    let splash = [screen::SPLASH, screen::SPLASH_WIDE].iter().any(|s| name.eq_ignore_ascii_case(s));
    let pause = name.eq_ignore_ascii_case(screen::PAUSE_MENU);
    let Some(frame) = Frame::find(tree, name, footer, assets) else { return Vec::new() };
    let settings = [
        screen::OPTIONS,
        screen::PAUSE_OPTIONS,
        screen::CONTROLS,
        screen::PAUSE_CONTROLS,
        screen::SAFE_HOUSE_RACE_SHEET,
        screen::SAFE_HOUSE_MILESTONES,
        screen::SAFE_HOUSE_BOUNTY,
    ]
    .iter()
    .any(|s| name.eq_ignore_ascii_case(s));
    legacy::hide(
        tree,
        splash,
        name.eq_ignore_ascii_case("Quick_Race_Brief.fng"),
        settings,
        name.eq_ignore_ascii_case(screen::EA_TRAX_JUKEBOX),
    );
    if footer.is_garage() {
        super::garage_ui::prompts::hide_legacy(tree, footer);
    }
    if name.eq_ignore_ascii_case(screen::SAFE_HOUSE_RIVAL_BIO) {
        legacy::hide_children(tree, 0xdde3_db34);
    }
    if [screen::SAFE_HOUSE_RACE_SHEET, screen::SAFE_HOUSE_MILESTONES, screen::SAFE_HOUSE_BOUNTY]
        .iter()
        .any(|s| name.eq_ignore_ascii_case(s))
    {
        legacy::hide_children(tree, 0x84bc_f2b5);
        legacy::hide_roots(tree, &[0xc0a6_849b, 0x12fd_7d4a, 0xfb80_cdac]);
    }
    let first_hint = tree.nodes.len();
    let hints: Vec<(Vec<Prompt>, String, Option<Action>)> = hint_set(name, footer, bindings, device)
        .into_iter()
        .map(|(keys, label, action)| (keys, label.text(), action))
        .collect();
    let colour =
        match pause || [screen::PAUSE_OPTIONS, screen::PAUSE_CONTROLS].iter().any(|s| name.eq_ignore_ascii_case(s)) {
            true => [255, 174, 64, 255],
            false => [255; 4],
        };
    let total: f32 = hints.iter().map(|(keys, label, _)| hint_width(keys, label, assets)).sum();
    let fit = (frame.width / total.max(1.0)).min((frame.height - 2.0) / 25.0).min(1.0);
    let mut x = match frame.centred {
        true => -total * fit * 0.5,
        false => -frame.width * 0.5,
    };
    let mut areas = Vec::new();
    for (keys, label, pressed) in hints {
        let start = x;
        let first = tree.nodes.len();
        for key in keys {
            let key_width = prompt_width(&key, assets) * fit;
            match key {
                Prompt::Icon(glyph) => image(tree, glyph, x + key_width * 0.5, 0.0, 25.0 * fit),
                Prompt::Key(key) => keycap(tree, &key, x, key_width, fit, colour, assets),
            }
            x += key_width;
        }
        text(tree, &label, x + 3.0 * fit, 0.0, fit, colour, assets);
        x += (text_width(&label, assets) + 22.0) * fit;
        if let Some(action) = pressed {
            areas.push((hint_area(action, frame.world, start, x, HINT_HALF_HEIGHT * fit), first..tree.nodes.len()));
        }
    }
    frame.apply(tree, first_hint);
    areas.into_iter().filter_map(|(area, nodes)| tree.nodes[nodes].iter().any(|n| n.visible).then_some(area)).collect()
}

#[cfg(test)]
#[path = "prompt_hit_tests.rs"]
mod hit_tests;

fn hint_area(action: Action, world: Mat4, start: f32, end: f32, half: f32) -> HintArea {
    let first = world.transform_point3(Vec3::new(start, -half, 0.0)).truncate();
    let mut min = first;
    let mut max = first;
    for (x, y) in [(start, half), (end, -half), (end, half)] {
        let point = world.transform_point3(Vec3::new(x, y, 0.0)).truncate();
        min = min.min(point);
        max = max.max(point);
    }
    HintArea { action, min, max }
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
    font.layout_scaled(
        text,
        TextStyle { justification: 0, leading: 0, max_width: 0 },
        assets.texture_size(font.texture_hash).unwrap_or((256, 256)),
        assets.font_scale(font.hash),
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
        let layout = font.layout_scaled(
            text,
            TextStyle { justification: 4, leading: 0, max_width: 0 },
            assets.texture_size(font.texture_hash).unwrap_or((256, 256)),
            assets.font_scale(font.hash),
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
