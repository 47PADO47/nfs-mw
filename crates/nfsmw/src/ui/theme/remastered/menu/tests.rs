//! Tests for the Remastered vertical menu: what it builds from a list, with no game data and no GPU.

use blackbox_ui::{BuildCx, DrawList, LayoutCx, NodeKind, Rect, Theme, UiNode};

use super::layout::{left_edge, row_y};
use crate::input::Action;
use crate::ui::input_icons::{ATLAS, Glyph};
use crate::ui::prompts::Prompt;
use crate::ui::theme::custom::{DriveFeedCell, palette};
use crate::ui::theme::remastered::{RemasteredTheme, palette_skin, skin};
use crate::ui::theme::{ElementId, MenuEntry, MenuHint, MenuList, MenuListCell, MenuStyle};

const ASPECT: f32 = 16.0 / 9.0;

fn entry(label: &str, enabled: bool) -> MenuEntry {
    MenuEntry { label: label.into(), enabled }
}

fn list(style: MenuStyle, selected: usize) -> MenuList {
    MenuList {
        title: "Main menu".into(),
        entries: vec![entry("Career", true), entry("Challenge Series", false), entry("Quick Race", true)],
        selected,
        first: 0,
        hints: vec![
            MenuHint { keys: vec![Prompt::Key("Enter".into())], label: "Accept", action: Some(Action::MenuAccept) },
            MenuHint { keys: vec![Prompt::Icon(Glyph::B)], label: "Back", action: Some(Action::MenuBack) },
        ],
        style,
        aspect: ASPECT,
    }
}

fn built(slot: ElementId, cell: &MenuListCell) -> Vec<UiNode> {
    let skin = palette::remastered().skin().merged(skin()).merged(palette_skin(&palette::remastered()));
    let theme = RemasteredTheme::new(DriveFeedCell::new(), cell.clone());
    let mut menu = theme.element(slot.slot(), &BuildCx::new(&skin)).expect("Remastered draws this menu");
    menu.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    menu.build(&mut out);
    out.iter().cloned().collect()
}

fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

fn text_at<'a>(nodes: &'a [UiNode], text: &str) -> &'a UiNode {
    nodes.iter().find(|n| n.text.as_deref() == Some(text)).unwrap_or_else(|| panic!("no {text:?} drawn"))
}

fn images_with(nodes: &[UiNode], wanted: u32) -> usize {
    nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture, .. } if texture == wanted)).count()
}

#[test]
fn the_four_icon_menus_are_vertical_and_the_rest_fall_back() {
    let theme = RemasteredTheme::default();
    let skin = skin();
    let cx = BuildCx::new(&skin);
    for slot in [ElementId::MainMenu, ElementId::Categories, ElementId::PauseMenu, ElementId::SafeHouseMenu] {
        assert!(theme.element(slot.slot(), &cx).is_some(), "{} is vertical", slot.name());
    }
    for slot in [ElementId::OptionTable, ElementId::TrackSelect, ElementId::Blacklist] {
        assert!(theme.element(slot.slot(), &cx).is_none(), "{} stays Original", slot.name());
    }
}

#[test]
fn a_slot_with_no_list_draws_nothing() {
    assert!(built(ElementId::MainMenu, &MenuListCell::new()).is_empty());
}

#[test]
fn the_title_and_the_entries_are_capitals_in_one_column_on_the_reference_rows() {
    let cell = MenuListCell::new();
    cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 0));
    let nodes = built(ElementId::MainMenu, &cell);
    assert!(texts(&nodes).contains(&"MAIN MENU"));
    for (i, label) in ["CAREER", "CHALLENGE SERIES", "QUICK RACE"].iter().enumerate() {
        let node = text_at(&nodes, label);
        assert!((node.world.w_axis.y - row_y(i)).abs() < 1e-3, "{label} sits on its row");
        assert!(node.world.w_axis.x > left_edge(ASPECT), "{label} is inside the screen");
    }
    let x = |label: &str| text_at(&nodes, label).world.w_axis.x;
    assert_eq!(x("CAREER"), x("QUICK RACE"), "one left edge for every entry");
}

#[test]
fn only_the_selected_entry_has_the_gold_bar_and_a_disabled_one_is_dimmed() {
    let cell = MenuListCell::new();
    cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 2));
    let nodes = built(ElementId::MainMenu, &cell);
    let on_row = |i: usize| {
        nodes
            .iter()
            .filter(|n| {
                matches!(n.kind, NodeKind::Image { texture: 0, .. }) && (n.world.w_axis.y - row_y(i)).abs() < 0.5
            })
            .count()
    };
    assert!(on_row(2) > 20, "the bar's strips are on the selected row");
    assert_eq!(on_row(0), 0, "and no other row has them");
    let career = text_at(&nodes, "CAREER").colour;
    assert!(text_at(&nodes, "CHALLENGE SERIES").colour[3] < career[3], "a disabled entry is fainter");
    assert!(text_at(&nodes, "QUICK RACE").colour[0] >= career[0], "the selected entry is the brightest");
}

#[test]
fn the_logo_and_the_tagline_depend_on_the_menu() {
    let logo = blackbox_feng::resource_handle("MW_Logo.tga");
    let cell = MenuListCell::new();
    for (style, has_logo, tagline) in
        [(MenuStyle::Main, true, true), (MenuStyle::Sub, true, false), (MenuStyle::Pause, false, false)]
    {
        cell.set(ElementId::MainMenu.slot(), list(style, 0));
        let nodes = built(ElementId::MainMenu, &cell);
        assert_eq!(images_with(&nodes, logo) == 1, has_logo, "{style:?} logo");
        assert_eq!(texts(&nodes).contains(&"RACE YOUR WAY"), tagline, "{style:?} tagline");
    }
}

#[test]
fn the_footer_draws_each_hint_with_its_key_and_label() {
    let cell = MenuListCell::new();
    cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 0));
    let nodes = built(ElementId::MainMenu, &cell);
    let all = texts(&nodes);
    assert!(all.contains(&"ENTER") && all.contains(&"Accept") && all.contains(&"Back"), "{all:?}");
    assert_eq!(images_with(&nodes, ATLAS), 1, "the pad glyph of the Back hint");
    let mut bare = list(MenuStyle::Main, 0);
    bare.hints.clear();
    cell.set(ElementId::MainMenu.slot(), bare);
    assert!(!texts(&built(ElementId::MainMenu, &cell)).contains(&"Accept"), "no hints, no footer");
}

#[test]
fn a_menu_is_placed_from_the_left_edge_of_the_screen() {
    let cell = MenuListCell::new();
    let mut menu = list(MenuStyle::Pause, 0);
    cell.set(ElementId::PauseMenu.slot(), menu.clone());
    let wide = text_at(&built(ElementId::PauseMenu, &cell), "CAREER").world.w_axis.x;
    menu.aspect = 4.0 / 3.0;
    cell.set(ElementId::PauseMenu.slot(), menu);
    let narrow = text_at(&built(ElementId::PauseMenu, &cell), "CAREER").world.w_axis.x;
    assert!((wide - narrow - (left_edge(ASPECT) - left_edge(4.0 / 3.0))).abs() < 1e-3);
}

#[test]
fn a_long_list_shows_a_window_of_it_and_no_scroll_bar() {
    let cell = MenuListCell::new();
    let mut long = list(MenuStyle::Sub, 5);
    long.entries = (0..12).map(|i| entry(&format!("Item {i}"), true)).collect();
    long.first = 3;
    cell.set(ElementId::Categories.slot(), long);
    let nodes = built(ElementId::Categories, &cell);
    let all = texts(&nodes);
    assert!(all.contains(&"ITEM 3") && all.contains(&"ITEM 10"), "eight entries from the window's start: {all:?}");
    assert!(!all.contains(&"ITEM 2") && !all.contains(&"ITEM 11"), "none outside the window: {all:?}");
    assert!(
        (text_at(&nodes, "ITEM 3").world.w_axis.y - row_y(0)).abs() < 1e-3,
        "the window's first entry is the first row"
    );
    let thin = nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture: 0, .. }) && n.world.x_axis.x < 3.0);
    assert_eq!(thin.filter(|n| n.world.y_axis.y > 100.0).count(), 0, "no tall thin bar beside the list");
}

#[test]
fn the_title_is_bigger_and_bold_in_the_accent_colour() {
    let cell = MenuListCell::new();
    cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 0));
    let nodes = built(ElementId::MainMenu, &cell);
    let title = text_at(&nodes, "MAIN MENU");
    let entry = text_at(&nodes, "CAREER");
    assert!(title.world.y_axis.y > entry.world.y_axis.y, "the title is set larger than the entries");
    let NodeKind::Text { font, .. } = title.kind else { panic!("the title is text") };
    assert_eq!(font, crate::ui::raster_text::RASTER_MENU_TITLE_FONT_KEY, "in the bold face");
    let accent = palette::remastered().secondary;
    assert_eq!(title.colour, accent, "and in the palette's accent");
}
