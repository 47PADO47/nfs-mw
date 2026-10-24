//! Contextual action hints, shared by every implemented front-end screen.

use blackbox_feng::{NodeKind, UiNode, UiTree, fe_hash_upper};
use glam::{Mat4, Quat, Vec2, Vec3};

use super::logic::Footer;
use super::{ids::screen, prompt_layout::Frame};
use crate::career::CareerHandle;
use crate::i18n::{self, t};
use crate::input::{Action, Bindings, InputDevice};
use crate::ui::{
    UiAssets,
    input_icons::{ATLAS, Glyph},
    prompts::{self, Prompt},
};

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

/// What a footer hint says it does. Most are the game's own hint words; the rest are the catalogue's (`prompts-*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintLabel {
    Accept,
    Back,
    Quit,
    Defaults,
    ViewTutorial,
    Showcase,
    Zoom,
    ViewCar,
    Continue,
    GameStats,
    Select,
    Column,
    Remap,
    ChangeMode,
    ViewRival,
    Randomize,
    Adjust,
    Done,
    // The garage's own hints.
    AddNewCar,
    CustomizeCar,
    DeleteCar,
    Backroom,
    MaxPerformance,
    AddToCart,
    ViewCart,
    RemoveNumbers,
    Remove,
    Add,
    Checkout,
    ClearAll,
    Cancel,
    Help,
}

impl HintLabel {
    /// The label's text in the current language.
    pub fn text(self) -> String {
        let label = match self {
            Self::Accept => 0x2B65_A216,         // H_ACCEPT
            Self::Back => 0x61D5_96D7,           // H_BACK
            Self::Quit => 0xC51F_BF74,           // PC_QUIT
            Self::Defaults => 0x00CA_369E,       // H_DEFAULTS
            Self::ViewTutorial => 0xCE39_D54D,   // VIEW_TUTORIAL
            Self::Showcase => 0xD616_E103,       // H_SHOWCASE
            Self::Zoom => 0xC524_9616,           // PC_ZOOM
            Self::ViewCar => 0x5DE7_9F0B,        // SHOWCASE_VIEW_CAR
            Self::Continue => 0x8098_A54C,       // COMMON_CONTINUE
            Self::GameStats => 0x7F73_40AF,      // COMMON_GAME_STATS
            Self::Select => 0x3FC3_2947,         // COMMON_SELECT
            Self::Randomize => 0x363D_108F,      // H_RANDOMIZE
            Self::Done => 0x8CD6_BD6D,           // COMMON_DONE
            Self::AddNewCar => 0xC259_8BD8,      // MYCAR_ADD_CAR
            Self::CustomizeCar => 0xC984_7935,   // MYCAR_EDIT_CAR
            Self::DeleteCar => 0xD5A1_1802,      // MYCAR_DELETE_CAR
            Self::Backroom => 0xEE11_7BF4,       // H_BACKROOM
            Self::MaxPerformance => 0x972C_3B80, // CM_MAX_PERF
            Self::AddToCart => 0xCFDF_1D5A,      // H_ADD_TO_CART
            Self::ViewCart => 0xE2DF_460A,       // H_VIEW_CART
            Self::RemoveNumbers => 0xBE43_4A38,  // CN_REMOVE_NUMBERS
            Self::Remove => 0x5DAB_CBC0,         // UNCHECK
            Self::Add => 0x028F_EADD,            // CHECK
            Self::Checkout => 0x2557_DE51,       // CSHOP_CHECKOUT
            Self::ClearAll => CLEAR_ALL,         // CSHOP_CLEAR
            Self::Cancel => 0x1A29_4DAD,         // COMMON_CANCEL
            Self::Help => 0x61D8_F34F,           // H_HELP
            Self::Column => return t!("prompts-column"),
            Self::Remap => return t!("prompts-remap"),
            Self::ChangeMode => return t!("prompts-change-mode"),
            Self::ViewRival => return t!("prompts-view-rival"),
            Self::Adjust => return t!("prompts-adjust"),
        };
        i18n::game_or_error(label)
    }
}

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
    match footer {
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
    if is(screen::EA_TRAX_JUKEBOX) {
        hints = vec![
            (prompts::pair(bindings, Action::MenuUp, Action::MenuDown, device), L::Select, None),
            (action(Action::MenuAccept), L::ChangeMode, Some(Action::MenuAccept)),
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
    let Some(frame) = Frame::find(tree, name, footer) else { return Vec::new() };
    hide_legacy_hints(tree, splash, name.eq_ignore_ascii_case("Quick_Race_Brief.fng"));
    if footer.is_garage() {
        super::garage_ui::prompts::hide_legacy(tree);
    }
    let first_hint = tree.nodes.len();
    let hints: Vec<(Vec<Prompt>, String, Option<Action>)> = hint_set(name, footer, bindings, device)
        .into_iter()
        .map(|(keys, label, action)| (keys, label.text(), action))
        .collect();
    let colour = match pause || name.eq_ignore_ascii_case(screen::PAUSE_OPTIONS) {
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
            let half = HINT_HALF_HEIGHT * fit;
            let a = frame.world.transform_point3(Vec3::new(start, -half, 0.0)).truncate();
            let b = frame.world.transform_point3(Vec3::new(x, half, 0.0)).truncate();
            areas.push(HintArea { action, min: a.min(b), max: a.max(b) });
        }
    }
    frame.apply(tree, first_hint);
    areas
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

fn hide_legacy_hints(tree: &mut UiTree, splash: bool, brief: bool) {
    // 0xD646_3100 is the Game Stats (and Randomize) button group.
    // 0xB8A7_C6CE is the profile list's own Accept; 0x28FD_842D the main menu's Game Stats card header.
    let names = [
        0x6A21_8478,
        0x812A_09D4,
        0x7379_349B,
        0x5E4D_1BDC,
        0xD646_3100,
        0xB8A7_C6CE,
        0x28FD_842D,
        super::ids::QUIT_BUTTON,
        // The overview's legacy Back triangle is a direct footer child, outside the Back text group.
        0x1AC9_4CA9,
    ];
    // The draw tree does not keep the labels its strings came from, so a package's own hint is known by the text its
    // label has in the current language.
    let words: Vec<String> = LEGACY_HINTS.iter().filter_map(|label| i18n::game(*label)).collect();
    let mut hidden = vec![false; tree.nodes.len()];
    for (i, n) in tree.nodes.iter().enumerate() {
        let hint_text = n.text.as_deref().is_some_and(|t| words.iter().any(|word| word == t.trim()));
        // The brief's Randomize root has the generic name "Group", shared by unrelated screens.
        let brief_hint =
            brief && [super::prompt_layout::BRIEF_FOOTER, super::prompt_layout::BRIEF_RANDOMIZE].contains(&n.name_hash);
        hidden[i] = hint_text || names.contains(&n.name_hash) || brief_hint || (splash && n.name_hash == 0xC4DF_3FF2);
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
    // The composed menu slot copies whole trees (`ui/theme/original/menu.rs`) and reads `visible`, not `draw_order`,
    // so a hidden node is marked invisible as well as dropped from the draw order.
    for (node, hidden) in tree.nodes.iter_mut().zip(&hidden) {
        node.visible &= !hidden;
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
