//! Tests for the Custom menus: what each one builds from a fixed list, with no game data and no GPU.

use blackbox_ui::{BuildCx, DrawList, LayoutCx, NodeKind, Rect, Skin, Theme, UiNode};
use glam::Vec2;

use super::layout::MenuFrame;
use super::value::SEGMENTS;
use super::{MenuLook, dialog_row_at, hint_at};
use crate::input::Action;
use crate::ui::input_icons::{ATLAS, Glyph};
use crate::ui::prompts::Prompt;
use crate::ui::theme::custom::tokens::{self, Colours, CustomLook};
use crate::ui::theme::custom::{CustomFeedCell, CustomTheme, RadioFeedCell, palette};
use crate::ui::theme::{
    ElementId, MenuEntry, MenuHint, MenuList, MenuListCell, MenuNudge, MenuStyle, MenuValue, default_skin,
};

pub(super) const ASPECT: f32 = 16.0 / 9.0;

pub(super) fn hints() -> Vec<MenuHint> {
    vec![
        MenuHint { keys: vec![Prompt::Key("Enter".into())], label: "Accept".into(), action: Some(Action::MenuAccept) },
        MenuHint { keys: vec![Prompt::Icon(Glyph::B)], label: "Back".into(), action: Some(Action::MenuBack) },
    ]
}

pub(super) fn list(style: MenuStyle, selected: usize) -> MenuList {
    MenuList {
        title: "Main menu".into(),
        message: String::new(),
        entries: vec![
            MenuEntry::plain("Career", true),
            MenuEntry::plain("Challenge Series", false),
            MenuEntry::plain("Quick Race", true),
        ],
        selected,
        first: 0,
        hints: hints(),
        style,
        nudge: None,
        scroll: 0.0,
        aspect: ASPECT,
    }
}

fn options(selected: usize) -> MenuList {
    let choice = |text: &str, arrows| MenuValue::Choice { text: text.into(), arrows };
    let row = |label: &str, value| MenuEntry { value, ..MenuEntry::plain(label, true) };
    MenuList {
        title: "HUD".into(),
        message: "Centered map, balanced information".into(),
        entries: vec![
            row("Preset", choice("Balanced", true)),
            row("HUD volume", MenuValue::Slider(80)),
            row("Advanced graphics", choice("Open", false)),
        ],
        nudge: Some(MenuNudge { row: 0, forward: true, age: 0.05 }),
        ..list(MenuStyle::Sub, selected)
    }
}

fn dialog(selected: usize) -> MenuList {
    MenuList {
        title: "Quit".into(),
        message: "Quit the game?".into(),
        entries: vec![MenuEntry::plain("Yes", true), MenuEntry::plain("No", true)],
        ..list(MenuStyle::Dialog, selected)
    }
}

pub(super) fn skin(look: MenuLook) -> Skin {
    default_skin()
        .merged(palette::bevy().skin())
        .merged(tokens::skin(CustomLook::default(), palette::bevy().secondary))
        .merged(look.skin())
}

pub(super) fn theme(cell: &MenuListCell) -> CustomTheme {
    CustomTheme::new(RadioFeedCell::default(), CustomFeedCell::default(), Default::default()).with_menus(cell.clone())
}

pub(super) fn built_with(theme: &CustomTheme, slot: ElementId, look: MenuLook, time: f32) -> Vec<UiNode> {
    let skin = skin(look);
    let mut menu = theme.element(slot.slot(), &BuildCx::new(&skin)).expect("Custom draws this menu");
    menu.layout(&LayoutCx::new(&skin, time), Rect::SCREEN);
    let mut out = DrawList::new();
    menu.build(&mut out);
    out.iter().cloned().collect()
}

pub(super) fn built(slot: ElementId, list: MenuList) -> Vec<UiNode> {
    let cell = MenuListCell::new();
    cell.set(slot.slot(), list);
    built_with(&theme(&cell), slot, MenuLook::default(), 0.0)
}

pub(super) fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

pub(super) fn text_at<'a>(nodes: &'a [UiNode], text: &str) -> &'a UiNode {
    nodes.iter().find(|n| n.text.as_deref() == Some(text)).unwrap_or_else(|| panic!("no {text:?} drawn"))
}

/// The untextured panels in `colour` whose middle is on the row at `y`.
pub(super) fn panels_at(nodes: &[UiNode], colour: [u8; 4], y: f32) -> usize {
    nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Image { texture: 0, .. }) && n.colour == colour)
        .filter(|n| (n.world.w_axis.y - y).abs() < 0.5)
        .count()
}

pub(super) fn accent() -> [u8; 4] {
    palette::bevy().secondary
}

#[test]
fn the_custom_theme_draws_the_menus_and_leaves_the_track_select_and_career_screens() {
    let theme = theme(&MenuListCell::new());
    let skin = skin(MenuLook::default());
    let cx = BuildCx::new(&skin);
    for slot in super::SLOTS {
        assert!(theme.element(slot.slot(), &cx).is_some(), "{} is a Custom menu", slot.name());
    }
    for slot in [ElementId::ControlsList, ElementId::Jukebox, ElementId::Credits, ElementId::Blacklist] {
        assert!(super::SLOTS.contains(&slot), "{} is drawn since PR 11b", slot.name());
    }
    assert!(theme.element(ElementId::TrackSelect.slot(), &cx).is_none(), "the track select stays Original");
    for slot in [ElementId::CarLot, ElementId::RapSheet, ElementId::RegionUnlock, ElementId::MemoryCard, ElementId::Sms]
    {
        assert!(theme.element(slot.slot(), &cx).is_none(), "{} stays Original", slot.name());
    }
}

#[test]
fn a_slot_with_no_list_draws_nothing() {
    let cell = MenuListCell::new();
    assert!(built_with(&theme(&cell), ElementId::MainMenu, MenuLook::default(), 0.0).is_empty());
}

#[test]
fn the_title_rows_and_hints_are_the_list_in_one_column() {
    let nodes = built(ElementId::MainMenu, list(MenuStyle::Main, 0));
    assert_eq!(texts(&nodes), ["MAIN MENU", "Career", "Challenge Series", "Quick Race", "Enter", "Accept", "Back"]);
    let frame = MenuFrame::new(ASPECT, 1.0);
    for (i, label) in ["Career", "Challenge Series", "Quick Race"].iter().enumerate() {
        let node = text_at(&nodes, label);
        assert!((node.world.w_axis.y - frame.row_y(i)).abs() < 1e-3, "{label} sits on its row");
    }
    let x = |label: &str| text_at(&nodes, label).world.w_axis.x;
    assert_eq!(x("Career"), x("Quick Race"), "one left edge for every row");
    let glyphs = nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture, .. } if texture == ATLAS)).count();
    assert_eq!(glyphs, 1, "the controller glyph of the Back hint");
}

#[test]
fn only_the_selected_row_has_the_raised_plate_and_the_accent_bar() {
    let c = Colours::BRIEF;
    let frame = MenuFrame::new(ASPECT, 1.0);
    let nodes = built(ElementId::MainMenu, list(MenuStyle::Main, 2));
    assert_eq!(panels_at(&nodes, accent(), frame.row_y(2)), 1, "the bar on the selected row");
    assert_eq!(panels_at(&nodes, accent(), frame.row_y(0)), 0, "and on no other");
    assert!(panels_at(&nodes, c.surface_raised, frame.row_y(2)) > 0, "the raised plate under it");
    assert_eq!(panels_at(&nodes, c.surface_raised, frame.row_y(0)), 0);
    assert_eq!(text_at(&nodes, "Quick Race").colour, c.text, "the selected row is the primary colour");
    assert_eq!(text_at(&nodes, "Career").colour, c.text_secondary, "the others the secondary");
    let disabled = text_at(&nodes, "Challenge Series").colour;
    assert_eq!(disabled[..3], c.text_secondary[..3]);
    assert!(disabled[3] < c.text_secondary[3], "a disabled row is fainter");
}

#[test]
fn the_pause_menu_dims_the_whole_window_and_the_others_a_column() {
    let window = MenuFrame::new(ASPECT, 1.0).window();
    let widest = |nodes: &[UiNode]| nodes.iter().map(|n| n.world.x_axis.x).fold(0.0_f32, f32::max);
    let pause = built(ElementId::PauseMenu, list(MenuStyle::Pause, 0));
    assert!((widest(&pause) - window.w).abs() < 1e-3, "the scrim covers the frozen world");
    let main = built(ElementId::MainMenu, list(MenuStyle::Main, 0));
    assert!(widest(&main) < window.w / 2.0, "the list's scrim is a column down the left");
}

#[test]
fn a_settings_page_shows_its_values_chevrons_slider_and_description() {
    let nodes = built(ElementId::OptionTable, options(0));
    let shown = texts(&nodes);
    for text in ["HUD", "Preset", "Balanced", "\u{2039}", "\u{203A}", "80%", "Open"] {
        assert!(shown.contains(&text), "{text:?} is drawn");
    }
    assert!(shown.contains(&"Centered map, balanced information"), "the description line");
    assert_eq!(shown.iter().filter(|t| **t == "\u{2039}").count(), 1, "only a cycling value has chevrons");
    let frame = MenuFrame::new(ASPECT, 1.0);
    let column = frame.value_column(frame.row_y(1));
    let segments = nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Image { texture: 0, .. }))
        .filter(|n| (n.world.w_axis.y - column.y).abs() < 0.5 && n.world.w_axis.x < column.max_x())
        .filter(|n| n.world.w_axis.x > column.min_x())
        .count();
    assert_eq!(segments, SEGMENTS, "the slider's ten segments");
    let selected = text_at(&nodes, "\u{203A}");
    assert_eq!(selected.colour, accent(), "the selected row's chevrons are the accent");
}

#[test]
fn the_nudged_chevron_moves_out_for_a_moment() {
    let still = built(ElementId::OptionTable, MenuList { nudge: None, ..options(0) });
    let nudged = built(ElementId::OptionTable, options(0));
    let x = |nodes: &[UiNode], text: &str| text_at(nodes, text).world.w_axis.x;
    assert!(x(&nudged, "\u{203A}") > x(&still, "\u{203A}"), "the right chevron was pressed");
    assert_eq!(x(&nudged, "\u{2039}"), x(&still, "\u{2039}"), "the left one stays");
}

#[test]
fn a_list_longer_than_a_screenful_shows_eight_rows_and_a_scroll_track() {
    let entries = (0..12).map(|i| MenuEntry::plain(format!("Row {i}"), true)).collect();
    let long = MenuList { entries, first: 2, selected: 5, ..list(MenuStyle::Sub, 0) };
    let nodes = built(ElementId::Categories, long);
    let rows = texts(&nodes).iter().filter(|t| t.starts_with("Row ")).count();
    assert_eq!(rows, super::layout::MAX_ROWS);
    assert!(!texts(&nodes).contains(&"Row 1"), "the window starts at the first shown entry");
    let frame = MenuFrame::new(ASPECT, 1.0);
    assert_eq!(panels_at(&nodes, accent(), frame.row_y(3)), 1, "the bar on the selected row, in window terms");
    let track = nodes.iter().filter(|n| n.colour == Colours::BRIEF.rule).count();
    assert!(track >= 1, "the scroll track");
}

#[test]
fn the_dialog_is_a_centred_plate_with_its_answers() {
    let nodes = built(ElementId::Dialog, dialog(1));
    assert_eq!(texts(&nodes), ["QUIT", "Quit the game?", "Yes", "No", "Enter", "Accept", "Back"]);
    let yes = text_at(&nodes, "Yes");
    let no = text_at(&nodes, "No");
    assert_eq!(yes.world.w_axis.x, no.world.w_axis.x, "the answers are one under the other");
    assert!(no.world.w_axis.y > yes.world.w_axis.y);
    assert_eq!(no.colour, Colours::BRIEF.text, "the selected answer is the primary colour");
    assert_eq!(panels_at(&nodes, accent(), no.world.w_axis.y), 1, "the bar on the selected answer");
    let list = dialog(1);
    let hit = |point: Vec2| dialog_row_at(point, &list, ASPECT, 1.0);
    assert_eq!(hit(Vec2::new(yes.world.w_axis.x, yes.world.w_axis.y)), Some(0));
    assert_eq!(hit(Vec2::new(no.world.w_axis.x, no.world.w_axis.y)), Some(1));
    assert_eq!(hit(Vec2::new(0.0, -230.0)), None, "outside the plate");
}

#[test]
fn a_hint_click_presses_its_action() {
    let frame = MenuFrame::new(ASPECT, 1.0);
    let spots = super::footer::hint_spots(&hints(), &frame);
    assert_eq!(spots.len(), 2);
    assert!(spots[0].area.max_x() < spots[1].area.min_x(), "the hints follow each other");
    assert!(spots[1].area.max_x() <= frame.safe().max_x() + 4.0, "and end at the safe area's right edge");
    let at = |i: usize| Vec2::new(spots[i].area.x, spots[i].area.y);
    assert_eq!(hint_at(&hints(), at(0), ASPECT, 1.0), Some(Action::MenuAccept));
    assert_eq!(hint_at(&hints(), at(1), ASPECT, 1.0), Some(Action::MenuBack));
    assert_eq!(hint_at(&hints(), Vec2::ZERO, ASPECT, 1.0), None);
}

#[test]
fn the_selection_glides_to_a_new_row_unless_motion_is_reduced() {
    let frame = MenuFrame::new(ASPECT, 1.0);
    for (look, glides) in [(MenuLook::new(1.0, false), true), (MenuLook::new(1.0, true), false)] {
        let cell = MenuListCell::new();
        let theme = theme(&cell);
        cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 0));
        built_with(&theme, ElementId::MainMenu, look, 1.0);
        cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 2));
        let nodes = built_with(&theme, ElementId::MainMenu, look, 1.0 + super::glide::GLIDE_SECS / 3.0);
        assert_eq!(panels_at(&nodes, accent(), frame.row_y(2)) == 0, glides, "reduced motion {look:?}");
        let arrived = built_with(&theme, ElementId::MainMenu, look, 2.0);
        assert_eq!(panels_at(&arrived, accent(), frame.row_y(2)), 1, "the bar arrives");
    }
}

#[test]
fn the_text_size_spreads_the_rows_and_the_hud_scale_does_not_reach_the_menus() {
    let cell = MenuListCell::new();
    cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 0));
    let y = |look: MenuLook| {
        let nodes = built_with(&theme(&cell), ElementId::MainMenu, look, 0.0);
        text_at(&nodes, "Quick Race").world.w_axis.y - text_at(&nodes, "Career").world.w_axis.y
    };
    assert!(y(MenuLook::new(1.3, false)) > y(MenuLook::new(1.0, false)), "a larger text size spreads the rows");
}
